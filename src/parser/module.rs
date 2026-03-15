use super::*;
use crate::lexer::TokenKind;

impl<'a> Parser<'a> {
    /// Parse a defmodule, returning a list of modules (parent + any nested ones).
    /// `parent_name` is Some("Outer") when parsing a nested module inside Outer.
    pub(super) fn parse_module_group(
        &mut self,
        parent_name: Option<&str>,
    ) -> Result<Vec<Module>, ParserError> {
        let id = self.node_ids.next_module();

        self.expect(TokenKind::Defmodule, "defmodule")?;
        let local_name = self.parse_module_reference("module name")?;
        let name = match parent_name {
            Some(parent) => format!("{parent}.{local_name}"),
            None => local_name,
        };
        self.expect(TokenKind::Do, "do")?;

        let mut forms = Vec::new();
        let mut attributes = Vec::new();
        let mut functions = Vec::new();
        let mut nested_modules: Vec<Module> = Vec::new();

        while !self.check(TokenKind::End) {
            if self.is_at_end() {
                return Err(self.expected("module declaration"));
            }

            if self.check(TokenKind::Def) || self.check(TokenKind::Defp) {
                functions.push(self.parse_function()?);
                continue;
            }

            if self.current_starts_module_form() {
                let mut new_forms = self.parse_module_forms()?;
                forms.append(&mut new_forms);
                continue;
            }

            if self.check(TokenKind::At) {
                attributes.push(self.parse_module_attribute()?);
                continue;
            }

            // Nested defmodule: flatten into sibling modules with dotted name.
            if self.check(TokenKind::Defmodule) {
                let mut nested = self.parse_module_group(Some(&name))?;
                nested_modules.append(&mut nested);
                continue;
            }

            return Err(self.expected("module declaration"));
        }

        self.expect(TokenKind::End, "end")?;

        let mut result = vec![Module::with_id(id, name, forms, attributes, functions)];
        result.append(&mut nested_modules);
        Ok(result)
    }

    pub(super) fn parse_function(&mut self) -> Result<Function, ParserError> {
        let id = self.node_ids.next_function();

        let visibility = if self.match_kind(TokenKind::Def) {
            FunctionVisibility::Public
        } else if self.match_kind(TokenKind::Defp) {
            FunctionVisibility::Private
        } else {
            return Err(self.expected("def or defp"));
        };

        let name = self.expect_ident("function name")?;
        self.expect(TokenKind::LParen, "(")?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen, ")")?;

        if self.check(TokenKind::Arrow)
            && self
                .peek(1)
                .map(|token| token.kind() == TokenKind::Ident && token.lexeme() == "dynamic")
                .unwrap_or(false)
        {
            return Err(ParserError::at_current(
                "dynamic annotation is only allowed on parameters",
                self.current(),
            ));
        }

        let guard = if self.match_kind(TokenKind::When) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect(TokenKind::Do, "do")?;
        let body = self.parse_block_body()?;
        self.expect(TokenKind::End, "end")?;

        Ok(Function::with_id(id, name, visibility, params, guard, body))
    }

    fn current_starts_module_form(&self) -> bool {
        self.current().is_some_and(|token| {
            token.kind() == TokenKind::Ident
                && matches!(
                    token.lexeme(),
                    "alias"
                        | "import"
                        | "require"
                        | "use"
                        | "defstruct"
                        | "defprotocol"
                        | "defimpl"
                )
        })
    }

    /// Parse one or more module forms. Returns a Vec because `alias Foo.{Bar, Baz}` expands
    /// to multiple individual Alias forms.
    fn parse_module_forms(&mut self) -> Result<Vec<ModuleForm>, ParserError> {
        let form_name = self.expect_ident("module form")?;

        match form_name.as_str() {
            "alias" => self.parse_alias_forms(),
            "import" => self.parse_import_form().map(|f| vec![f]),
            "require" => self.parse_named_module_form("require").map(|f| vec![f]),
            "use" => self.parse_named_module_form("use").map(|f| vec![f]),
            "defstruct" => self.parse_defstruct_form().map(|f| vec![f]),
            "defprotocol" => self.parse_defprotocol_form().map(|f| vec![f]),
            "defimpl" => self.parse_defimpl_form().map(|f| vec![f]),
            _ => Err(ParserError::at_current(
                format!("unsupported module form '{form_name}'"),
                self.current(),
            )),
        }
    }

    /// Parse alias form(s). Supports both:
    ///   `alias Foo.Bar` / `alias Foo.Bar, as: B`
    ///   `alias Foo.{Bar, Baz}` (multi-alias, expands to two Alias forms)
    fn parse_alias_forms(&mut self) -> Result<Vec<ModuleForm>, ParserError> {
        // `parse_module_reference` stops before `.{`, so for `alias Foo.{Bar, Baz}`
        // it returns "Foo" and leaves ".{" in the token stream.
        let base = self.parse_module_reference("aliased module")?;

        // Detect multi-alias: Foo.{Bar, Baz} — current token is `.` next is `{`
        if self.check(TokenKind::Dot) && self.peek(1).is_some_and(|t| t.kind() == TokenKind::LBrace)
        {
            self.advance(); // consume `.`
            return self.parse_multi_alias(base);
        }

        // Single alias with optional `, as: Name`
        let mut as_name = base.rsplit('.').next().unwrap_or(&base).to_string();

        if self.match_kind(TokenKind::Comma) {
            let option_token = self.expect_token(TokenKind::Ident, "alias option")?;
            if option_token.lexeme() != "as" {
                return Err(ParserError::at_current(
                    format!(
                        "unsupported alias option '{}'; supported syntax: alias Module, as: Name",
                        option_token.lexeme()
                    ),
                    Some(option_token),
                ));
            }

            self.expect(TokenKind::Colon, ":")?;
            as_name = self.expect_ident("alias name")?;
        }

        Ok(vec![ModuleForm::Alias {
            module: base,
            as_name,
        }])
    }

    /// Parse `{Child1, Child2}` after base module prefix, producing one Alias per child.
    fn parse_multi_alias(&mut self, base: String) -> Result<Vec<ModuleForm>, ParserError> {
        self.expect(TokenKind::LBrace, "{")?;

        let mut forms = Vec::new();

        loop {
            let child = self.expect_ident("alias module name")?;
            let full_module = format!("{base}.{child}");
            forms.push(ModuleForm::Alias {
                as_name: child,
                module: full_module,
            });

            if self.match_kind(TokenKind::Comma) {
                // Allow trailing comma before `}`
                if self.check(TokenKind::RBrace) {
                    break;
                }
                continue;
            }
            break;
        }

        self.expect(TokenKind::RBrace, "}")?;
        Ok(forms)
    }

    fn parse_import_form(&mut self) -> Result<ModuleForm, ParserError> {
        let module = self.parse_module_reference("module name")?;
        let mut only = None;
        let mut except = None;

        if self.match_kind(TokenKind::Comma) {
            let option_token = self.expect_token(TokenKind::Ident, "import option")?;
            let option_name = option_token.lexeme();
            if !matches!(option_name, "only" | "except") {
                return Err(ParserError::at_current(
                    format!(
                        "unsupported import option '{}'; supported syntax: import Module, only: [name: arity] or except: [name: arity]",
                        option_name
                    ),
                    Some(option_token),
                ));
            }

            self.expect(TokenKind::Colon, ":")?;
            let entries = self.parse_import_filter_entries(option_name)?;

            match option_name {
                "only" => only = Some(entries),
                "except" => except = Some(entries),
                _ => unreachable!("validated import option"),
            }

            if self.match_kind(TokenKind::Comma) {
                return Err(ParserError::at_current(
                    "import accepts exactly one filter option (`only:` or `except:`)",
                    self.current(),
                ));
            }
        }

        Ok(ModuleForm::Import {
            module,
            only,
            except,
        })
    }

    fn parse_import_filter_entries(
        &mut self,
        option_name: &str,
    ) -> Result<Vec<ImportFunctionSpec>, ParserError> {
        self.expect(TokenKind::LBracket, "[")?;

        let mut entries = Vec::new();
        let mut seen = std::collections::HashSet::new();
        if self.match_kind(TokenKind::RBracket) {
            return Ok(entries);
        }

        loop {
            let function_name = self
                .expect_token(TokenKind::Ident, "import filter function name")
                .map_err(|_| self.invalid_import_filter_shape(option_name))?
                .lexeme()
                .to_string();
            self.expect(TokenKind::Colon, ":")
                .map_err(|_| self.invalid_import_filter_shape(option_name))?;
            let arity_token = self
                .expect_token(TokenKind::Integer, "import filter arity")
                .map_err(|_| self.invalid_import_filter_shape(option_name))?;
            let arity = arity_token
                .lexeme()
                .parse::<usize>()
                .map_err(|_| self.invalid_import_filter_shape(option_name))?;

            if seen.insert((function_name.clone(), arity)) {
                entries.push(ImportFunctionSpec {
                    name: function_name,
                    arity,
                });
            }

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        self.expect(TokenKind::RBracket, "]")
            .map_err(|_| self.invalid_import_filter_shape(option_name))?;

        Ok(entries)
    }

    fn invalid_import_filter_shape(&self, option_name: &str) -> ParserError {
        ParserError::at_current(
            format!(
                "invalid import {option_name} option; expected {option_name}: [name: arity, ...]"
            ),
            self.current(),
        )
    }

    fn parse_named_module_form(&mut self, form_name: &str) -> Result<ModuleForm, ParserError> {
        let module = self.parse_module_reference("module name")?;

        if self.match_kind(TokenKind::Comma) {
            let option_token = self.expect_token(TokenKind::Ident, "module form option")?;
            return Err(ParserError::at_current(
                format!(
                    "unsupported {form_name} option '{}'; remove options from {form_name} for now",
                    option_token.lexeme()
                ),
                Some(option_token),
            ));
        }

        let form = match form_name {
            "require" => ModuleForm::Require { module },
            "use" => ModuleForm::Use { module },
            _ => {
                return Err(ParserError::at_current(
                    format!("unsupported module form '{form_name}'"),
                    self.current(),
                ));
            }
        };

        Ok(form)
    }

    fn parse_defstruct_form(&mut self) -> Result<ModuleForm, ParserError> {
        let mut fields = Vec::new();

        loop {
            let name = self.expect_ident("struct field")?;
            self.expect(TokenKind::Colon, ":")?;
            let default = self.parse_expression()?;
            fields.push(StructFieldEntry { name, default });

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        Ok(ModuleForm::Defstruct { fields })
    }

    fn parse_defprotocol_form(&mut self) -> Result<ModuleForm, ParserError> {
        let name = self.parse_module_reference("protocol name")?;
        self.expect(TokenKind::Do, "do")?;

        let mut functions = Vec::new();
        while !self.check(TokenKind::End) {
            if self.is_at_end() {
                return Err(self.expected("protocol declaration"));
            }
            functions.push(self.parse_protocol_signature()?);
        }

        self.expect(TokenKind::End, "end")?;

        Ok(ModuleForm::Defprotocol { name, functions })
    }

    fn parse_protocol_signature(&mut self) -> Result<ProtocolFunctionSignature, ParserError> {
        self.expect(TokenKind::Def, "def")?;
        let name = self.expect_ident("protocol function name")?;
        self.expect(TokenKind::LParen, "(")?;

        let mut params = Vec::new();
        if !self.check(TokenKind::RParen) {
            loop {
                params.push(self.expect_ident("protocol function parameter")?);
                if self.match_kind(TokenKind::Comma) {
                    continue;
                }
                break;
            }
        }

        self.expect(TokenKind::RParen, ")")?;

        if self.check(TokenKind::Do) {
            return Err(ParserError::at_current(
                "protocol declarations must not include function bodies",
                self.current(),
            ));
        }

        Ok(ProtocolFunctionSignature { name, params })
    }

    fn parse_defimpl_form(&mut self) -> Result<ModuleForm, ParserError> {
        let protocol = self.parse_module_reference("protocol name")?;
        self.expect(TokenKind::Comma, ",")?;

        if !self.check(TokenKind::For) {
            return Err(self.expected("for"));
        }
        self.advance();
        self.expect(TokenKind::Colon, ":")?;
        let target = self.parse_module_reference("implementation target")?;

        if self.match_kind(TokenKind::Comma) {
            let option = self.expect_ident("defimpl option")?;
            return Err(ParserError::at_current(
                format!(
                    "unsupported defimpl option '{option}'; only `for:` is currently supported"
                ),
                self.current(),
            ));
        }

        self.expect(TokenKind::Do, "do")?;

        let mut functions = Vec::new();
        while !self.check(TokenKind::End) {
            if self.is_at_end() {
                return Err(self.expected("defimpl function declaration"));
            }

            let function = self.parse_function()?;
            if function.is_private() {
                return Err(ParserError::at_current(
                    "defimpl functions must be public (def)",
                    self.current(),
                ));
            }

            functions.push(ProtocolImplFunction {
                name: function.name,
                params: function.params,
                guard: function.guard,
                body: function.body,
            });
        }

        self.expect(TokenKind::End, "end")?;

        Ok(ModuleForm::Defimpl {
            protocol,
            target,
            functions,
        })
    }

    pub(super) fn parse_module_attribute(&mut self) -> Result<ModuleAttribute, ParserError> {
        self.expect(TokenKind::At, "@")?;
        let name = self.expect_ident("attribute name")?;
        let value = self.parse_expression()?;

        Ok(ModuleAttribute { name, value })
    }

    pub(super) fn parse_module_reference(&mut self, expected: &str) -> Result<String, ParserError> {
        let mut module = self.expect_ident(expected)?;

        // Only consume `.` when followed by an ident (not `{` or other tokens).
        // This allows `alias Foo.{Bar, Baz}` to leave the `.{` for the caller to handle.
        while self.check(TokenKind::Dot)
            && self.peek(1).is_some_and(|t| t.kind() == TokenKind::Ident)
        {
            self.advance(); // consume `.`
            let segment = self.expect_ident("module name segment")?;
            module.push('.');
            module.push_str(&segment);
        }

        Ok(module)
    }

    pub(super) fn parse_params(&mut self) -> Result<Vec<Parameter>, ParserError> {
        let mut params = Vec::new();
        let mut saw_default = false;

        if self.check(TokenKind::RParen) {
            return Ok(params);
        }

        loop {
            let param = self.parse_param(params.len())?;

            if saw_default && !param.has_default() {
                return Err(ParserError::at_current(
                    "default parameters must be trailing",
                    self.current(),
                ));
            }
            saw_default |= param.has_default();
            params.push(param);

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        Ok(params)
    }

    fn parse_param(&mut self, index: usize) -> Result<Parameter, ParserError> {
        let (name, annotation, pattern, supports_default) =
            if self.current_starts_dynamic_param_annotation() {
                self.advance();
                let name = self.expect_ident("parameter name")?;
                (
                    name.clone(),
                    ParameterAnnotation::Dynamic,
                    Pattern::Bind { name },
                    true,
                )
            } else {
                let pattern = self.parse_pattern()?;
                let supports_default = matches!(pattern, Pattern::Bind { .. });
                let name = match &pattern {
                    Pattern::Bind { name } => name.clone(),
                    _ => format!("__arg{index}"),
                };
                (
                    name,
                    ParameterAnnotation::Inferred,
                    pattern,
                    supports_default,
                )
            };

        let default = if self.match_kind(TokenKind::BackslashBackslash) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        if default.is_some() && !supports_default {
            return Err(ParserError::at_current(
                "default values require variable parameters",
                self.current(),
            ));
        }

        match annotation {
            ParameterAnnotation::Inferred => Ok(Parameter::inferred(name, pattern, default)),
            ParameterAnnotation::Dynamic => Ok(Parameter::dynamic(name, default)),
        }
    }

    fn current_starts_dynamic_param_annotation(&self) -> bool {
        let Some(current) = self.current() else {
            return false;
        };

        if current.kind() != TokenKind::Ident || current.lexeme() != "dynamic" {
            return false;
        }

        self.peek(1)
            .map(|next| next.kind() == TokenKind::Ident)
            .unwrap_or(false)
    }

    pub(super) fn current_starts_module_reference(&self) -> bool {
        self.current().is_some_and(|token| {
            token.kind() == TokenKind::Ident && starts_with_uppercase(token.lexeme())
        })
    }
}
