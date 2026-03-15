use super::*;
use crate::lexer::TokenKind;

impl<'a> Parser<'a> {
    /// Parse alias form(s). Supports both:
    ///   `alias Foo.Bar` / `alias Foo.Bar, as: B`
    ///   `alias Foo.{Bar, Baz}` (multi-alias, expands to two Alias forms)
    pub(super) fn parse_alias_forms(&mut self) -> Result<Vec<ModuleForm>, ParserError> {
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

    pub(super) fn parse_import_form(&mut self) -> Result<ModuleForm, ParserError> {
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
}
