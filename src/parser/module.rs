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

        let module_span = self.expect_token(TokenKind::Defmodule, "defmodule")?.span();
        let local_name = self.parse_module_reference("module name")?;
        let name = match parent_name {
            Some(parent) => format!("{parent}.{local_name}"),
            None => local_name,
        };
        let construct = format!("module '{name}'");
        let hint = format!("add 'do' after 'defmodule {name}' to begin the module body");
        self.expect_block_do(&construct, module_span, hint)?;

        let mut forms = Vec::new();
        let mut attributes = Vec::new();
        let mut functions = Vec::new();
        let mut nested_modules: Vec<Module> = Vec::new();

        while !self.check(TokenKind::End) {
            if self.is_at_end() {
                return Err(self.missing_end_error(&construct, module_span));
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

        self.expect_block_end(&construct, module_span)?;

        let mut result = vec![Module::with_id(id, name, forms, attributes, functions)];
        result.append(&mut nested_modules);
        Ok(result)
    }

    pub(super) fn parse_function(&mut self) -> Result<Function, ParserError> {
        let id = self.node_ids.next_function();

        let function_span = if self.match_kind(TokenKind::Def) {
            self.tokens[self.index - 1].span()
        } else if self.match_kind(TokenKind::Defp) {
            self.tokens[self.index - 1].span()
        } else {
            return Err(self.expected("def or defp"));
        };

        let visibility = match self.tokens[self.index - 1].kind() {
            TokenKind::Def => FunctionVisibility::Public,
            TokenKind::Defp => FunctionVisibility::Private,
            _ => unreachable!("validated def/defp token should determine function visibility"),
        };

        let name = self.expect_ident("function name")?;
        self.expect(TokenKind::LParen, "(")?;
        let params = self.parse_params(name.as_str())?;
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

        let construct = format!("function '{name}'");
        let hint = format!(
            "add 'do' after the function signature for '{name}' to begin the function body"
        );
        self.expect_block_do(&construct, function_span, hint)?;
        let body = self.parse_block_body()?;
        self.expect_block_end(&construct, function_span)?;

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
                if !self.check(TokenKind::RParen) && self.current_starts_missing_param_comma() {
                    return Err(self.missing_comma_error(
                        "protocol parameter list",
                        format!(
                            "separate protocol parameters with commas, for example `def {name}(left, right)`"
                        ),
                    ));
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

    pub(super) fn parse_params(
        &mut self,
        function_name: &str,
    ) -> Result<Vec<Parameter>, ParserError> {
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

            if !self.check(TokenKind::RParen) && self.current_starts_missing_param_comma() {
                return Err(self.missing_comma_error(
                    "function parameter list",
                    format!(
                        "separate parameters with commas, for example `def {function_name}(left, right) do ... end`"
                    ),
                ));
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
