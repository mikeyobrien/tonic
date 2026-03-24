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
        let opening_span = self.expect_token(TokenKind::LBrace, "{")?.span();

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

            if !self.check(TokenKind::RBrace) && self.current_starts_missing_alias_child_comma() {
                return Err(self.missing_comma_error(
                    "alias child list",
                    format!(
                        "separate alias children with commas, for example `alias {base}.{{Bar, Baz}}`"
                    ),
                ));
            }

            break;
        }

        if !self.check(TokenKind::RBrace) && self.current_starts_module_item_boundary() {
            return Err(self.unclosed_delimiter_error(
                "alias child list",
                "}",
                opening_span,
                format!(
                    "add '}}' to close the alias child list, for example `alias {base}.{{Bar, Baz}}`"
                ),
            ));
        }

        self.expect_closing_delimiter(
            TokenKind::RBrace,
            "}",
            "alias child list",
            opening_span,
            format!(
                "add '}}' to close the alias child list, for example `alias {base}.{{Bar, Baz}}`"
            ),
        )?;
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
        let opening_span = self.expect_token(TokenKind::LBracket, "[")?.span();

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

            if !self.check(TokenKind::RBracket) && self.current_starts_missing_keyword_entry_comma()
            {
                return Err(self.missing_comma_error(
                    &format!("import {option_name} filter list"),
                    format!(
                        "separate import {option_name} entries with commas, for example `import Enum, {option_name}: [map: 2, reduce: 3]`"
                    ),
                ));
            }

            break;
        }

        if !self.check(TokenKind::RBracket) && self.current_starts_module_item_boundary() {
            return Err(self.unclosed_delimiter_error(
                &format!("import {option_name} filter list"),
                "]",
                opening_span,
                format!(
                    "add ']' to close the import {option_name} filter list, for example `import Enum, {option_name}: [map: 2]`"
                ),
            ));
        }

        self.expect_closing_delimiter(
            TokenKind::RBracket,
            "]",
            &format!("import {option_name} filter list"),
            opening_span,
            format!(
                "add ']' to close the import {option_name} filter list, for example `import Enum, {option_name}: [map: 2]`"
            ),
        )?;

        Ok(entries)
    }

    fn current_starts_module_item_boundary(&self) -> bool {
        self.current().is_some_and(|token| {
            matches!(
                token.kind(),
                TokenKind::Def
                    | TokenKind::Defp
                    | TokenKind::Defmodule
                    | TokenKind::At
                    | TokenKind::End
                    | TokenKind::Eof
            ) || (token.kind() == TokenKind::Ident
                && matches!(
                    token.lexeme(),
                    "alias"
                        | "import"
                        | "require"
                        | "use"
                        | "defstruct"
                        | "defprotocol"
                        | "defimpl"
                ))
        })
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
