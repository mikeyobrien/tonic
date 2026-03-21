use super::*;
use crate::lexer::TokenKind;

impl<'a> Parser<'a> {
    pub(super) fn parse_interpolated_string_expression(&mut self) -> Result<Expr, ParserError> {
        let start_token = self
            .advance()
            .expect("string start token should be available");
        let offset = start_token.span().start();
        let mut segments = Vec::new();

        loop {
            if self.check(TokenKind::StringPart) {
                let token = self.advance().unwrap();
                segments.push(InterpolationSegment::String {
                    value: token.lexeme().to_string(),
                });
            } else if self.check(TokenKind::InterpolationStart) {
                self.advance().unwrap();
                let expr = self.parse_expression()?;
                self.expect(
                    TokenKind::InterpolationEnd,
                    "expected '}' after interpolated expression",
                )?;
                segments.push(InterpolationSegment::Expr { expr });
            } else if self.check(TokenKind::StringEnd) {
                self.advance().unwrap();
                break;
            } else {
                return Err(ParserError::at_current(
                    "unexpected token inside string interpolation",
                    self.peek(0),
                ));
            }
        }

        Ok(Expr::interpolated_string(
            self.node_ids.next_expr(),
            offset,
            segments,
        ))
    }

    pub(super) fn parse_bitstring_literal_expression(&mut self) -> Result<Expr, ParserError> {
        let opening_span = self.expect_token(TokenKind::LtLt, "<<")?.span();
        let offset = opening_span.start();

        let mut items = Vec::new();
        if !self.check(TokenKind::GtGt) {
            loop {
                items.push(self.parse_atomic_expression()?);

                if self.match_kind(TokenKind::Comma) {
                    continue;
                }

                if !self.check(TokenKind::GtGt)
                    && self.current_starts_missing_bitstring_item_comma()
                {
                    return Err(self.missing_comma_error(
                        "bitstring literal",
                        "separate bitstring elements with commas, for example `<<left, right>>`",
                    ));
                }

                break;
            }
        }

        self.expect_closing_delimiter(
            TokenKind::GtGt,
            ">>",
            "bitstring literal",
            opening_span,
            "add '>>' to close the bitstring literal, for example `<<left, right>>`",
        )?;

        Ok(Expr::bitstring(self.node_ids.next_expr(), offset, items))
    }

    pub(super) fn parse_tuple_literal_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::LBrace, "{")?.span().start();
        let items = self.parse_expression_items(TokenKind::RBrace, "}")?;
        Ok(Expr::tuple(self.node_ids.next_expr(), offset, items))
    }

    pub(super) fn parse_list_or_keyword_literal_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::LBracket, "[")?.span().start();

        if self.check(TokenKind::RBracket) {
            self.advance();
            return Ok(Expr::list(self.node_ids.next_expr(), offset, Vec::new()));
        }

        if self.starts_keyword_literal_entry() {
            let entries = self.parse_label_entries(TokenKind::RBracket, "keyword key")?;
            return Ok(Expr::keyword(self.node_ids.next_expr(), offset, entries));
        }

        let items = self.parse_expression_items(TokenKind::RBracket, "]")?;
        Ok(Expr::list(self.node_ids.next_expr(), offset, items))
    }

    pub(super) fn parse_percent_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::Percent, "%")?.span().start();

        if self.check(TokenKind::LBrace) {
            return self.parse_map_literal_expression_after_percent(offset);
        }

        self.parse_struct_literal_expression(offset)
    }

    fn parse_map_literal_expression_after_percent(
        &mut self,
        offset: usize,
    ) -> Result<Expr, ParserError> {
        self.expect(TokenKind::LBrace, "{")?;

        if self.match_kind(TokenKind::RBrace) {
            return Ok(Expr::map(self.node_ids.next_expr(), offset, Vec::new()));
        }

        if self.starts_keyword_literal_entry() {
            let entries = self.parse_map_entries_after_first()?;
            return Ok(Expr::map(self.node_ids.next_expr(), offset, entries));
        }

        let first_key = self.parse_expression()?;

        if self.match_kind(TokenKind::Pipe) {
            let entries = self.parse_label_entries(TokenKind::RBrace, "map update key")?;
            return Ok(Expr::map_update(
                self.node_ids.next_expr(),
                offset,
                first_key,
                entries,
            ));
        }

        let mut entries = vec![self.parse_map_entry_from_key(first_key)?];

        while self.match_kind(TokenKind::Comma) {
            entries.push(self.parse_map_entry()?);
        }

        if !self.check(TokenKind::RBrace) {
            return Err(self.closing_delimiter_error(TokenKind::RBrace, "}"));
        }
        self.advance();

        Ok(Expr::map(self.node_ids.next_expr(), offset, entries))
    }

    fn parse_struct_literal_expression(&mut self, offset: usize) -> Result<Expr, ParserError> {
        let module = self.parse_module_reference("struct module")?;
        self.expect(TokenKind::LBrace, "{")?;

        if self.match_kind(TokenKind::RBrace) {
            return Ok(Expr::struct_literal(
                self.node_ids.next_expr(),
                offset,
                module,
                Vec::new(),
            ));
        }

        if self.starts_keyword_literal_entry() {
            let entries = self.parse_label_entries(TokenKind::RBrace, "struct field")?;
            return Ok(Expr::struct_literal(
                self.node_ids.next_expr(),
                offset,
                module,
                entries,
            ));
        }

        let base = self.parse_expression()?;
        self.expect(TokenKind::Pipe, "|")?;
        let updates = self.parse_label_entries(TokenKind::RBrace, "struct update field")?;

        Ok(Expr::struct_update(
            self.node_ids.next_expr(),
            offset,
            module,
            base,
            updates,
        ))
    }

    pub(super) fn starts_keyword_literal_entry(&self) -> bool {
        self.check(TokenKind::Ident)
            && self
                .peek(1)
                .is_some_and(|token| token.kind() == TokenKind::Colon)
    }

    fn parse_map_entries_after_first(&mut self) -> Result<Vec<MapExprEntry>, ParserError> {
        let mut entries = vec![self.parse_map_entry_from_label()?];

        while self.match_kind(TokenKind::Comma) {
            entries.push(self.parse_map_entry()?);
        }

        if !self.check(TokenKind::RBrace) {
            return Err(self.closing_delimiter_error(TokenKind::RBrace, "}"));
        }
        self.advance();
        Ok(entries)
    }

    fn parse_map_entry(&mut self) -> Result<MapExprEntry, ParserError> {
        if self.starts_keyword_literal_entry() {
            return self.parse_map_entry_from_label();
        }

        let key = self.parse_expression()?;
        self.parse_map_entry_from_key(key)
    }

    fn parse_map_entry_from_label(&mut self) -> Result<MapExprEntry, ParserError> {
        let offset = self
            .current()
            .map(|token| token.span().start())
            .unwrap_or(0);
        let label = self.expect_ident("map key")?;
        self.expect(TokenKind::Colon, ":")?;
        let value = self.parse_expression()?;

        Ok(MapExprEntry {
            key: Expr::atom(self.node_ids.next_expr(), offset, label),
            value,
        })
    }

    pub(super) fn parse_map_entry_from_key(
        &mut self,
        key: Expr,
    ) -> Result<MapExprEntry, ParserError> {
        if !self.match_kind(TokenKind::FatArrow) {
            return Err(self.missing_map_fat_arrow_error(
                "map entry",
                "write `%{key => value}` for computed keys, or use `%{name: value}` when the key is an atom label",
            ));
        }

        let value = self.parse_expression()?;
        Ok(MapExprEntry { key, value })
    }

    pub(super) fn parse_label_entries(
        &mut self,
        closing: TokenKind,
        expected_key: &str,
    ) -> Result<Vec<LabelExprEntry>, ParserError> {
        let mut entries = Vec::new();

        loop {
            let key = self.expect_ident(expected_key)?;
            self.expect(TokenKind::Colon, ":")?;
            let value = self.parse_expression()?;
            entries.push(LabelExprEntry { key, value });

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        // Provide better error messages for maps/keyword lists
        if !self.check(closing) {
            return Err(self.closing_delimiter_error(closing, "}"));
        }
        self.advance();
        Ok(entries)
    }

    pub(super) fn parse_expression_items(
        &mut self,
        closing: TokenKind,
        expected_closing: &str,
    ) -> Result<Vec<Expr>, ParserError> {
        let mut items = Vec::new();

        if self.check(closing) {
            self.advance();
            return Ok(items);
        }

        loop {
            items.push(self.parse_expression()?);

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        // Provide better error messages instead of generic "expected ], found X"
        if !self.check(closing) {
            return Err(self.closing_delimiter_error(closing, expected_closing));
        }
        self.advance();
        Ok(items)
    }

    /// Generate a helpful error message when a closing delimiter is missing.
    ///
    /// Detects two common patterns:
    /// - Missing comma: the next token could start another expression
    /// - Unclosed delimiter: reached `end` or EOF without finding the closing delimiter
    fn closing_delimiter_error(&self, closing: TokenKind, expected_closing: &str) -> ParserError {
        if let Some(token) = self.current() {
            let kind = token.kind();

            // Pattern: missing comma — next token can start an expression
            if super::token_can_start_no_paren_arg(kind)
                || kind == TokenKind::Minus
                || kind == TokenKind::Not
            {
                let container = match closing {
                    TokenKind::RBracket => "list",
                    TokenKind::RBrace => "tuple/map",
                    TokenKind::RParen => "arguments",
                    _ => "expression",
                };
                return ParserError::at_current(
                    format!(
                        "[E0001] missing comma in {container}: expected ',' or '{expected_closing}', but found another expression. \
                         hint: add a comma between elements"
                    ),
                    self.current(),
                );
            }

            // Pattern: unclosed delimiter — hit `end` or block-level keyword
            if kind == TokenKind::End || kind == TokenKind::Eof {
                let container = match closing {
                    TokenKind::RBracket => "list '[' was never closed",
                    TokenKind::RBrace => "tuple/map '{' was never closed",
                    TokenKind::RParen => "parenthesis '(' was never closed",
                    _ => "delimiter was never closed",
                };
                return ParserError::at_current(
                    format!(
                        "[E0002] unclosed delimiter: {container}. \
                         hint: add '{expected_closing}' to close the expression"
                    ),
                    self.current(),
                );
            }
        } else {
            // EOF with no current token
            let container = match closing {
                TokenKind::RBracket => "list '[' was never closed",
                TokenKind::RBrace => "tuple/map '{' was never closed",
                TokenKind::RParen => "parenthesis '(' was never closed",
                _ => "delimiter was never closed",
            };
            return ParserError::at_current(
                format!(
                    "[E0002] unclosed delimiter: {container}. \
                     hint: add '{expected_closing}' before end of file"
                ),
                None,
            );
        }

        // Fallback: use generic message with error code
        self.expected(expected_closing)
    }
}
