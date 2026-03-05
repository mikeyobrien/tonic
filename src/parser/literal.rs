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
        let offset = self.expect_token(TokenKind::Lt, "<")?.span().start();
        self.expect(TokenKind::Lt, "<")?;

        let mut items = Vec::new();
        if !(self.check(TokenKind::Gt)
            && self
                .peek(1)
                .is_some_and(|token| token.kind() == TokenKind::Gt))
        {
            loop {
                items.push(self.parse_atomic_expression()?);

                if self.match_kind(TokenKind::Comma) {
                    continue;
                }

                break;
            }
        }

        self.expect(TokenKind::Gt, ">")?;
        self.expect(TokenKind::Gt, ">")?;

        Ok(Expr::list(self.node_ids.next_expr(), offset, items))
    }

    pub(super) fn parse_tuple_literal_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::LBrace, "{")?.span().start();
        let items = self.parse_expression_items(TokenKind::RBrace, "}")?;
        Ok(Expr::tuple(self.node_ids.next_expr(), offset, items))
    }

    pub(super) fn parse_list_or_keyword_literal_expression(
        &mut self,
    ) -> Result<Expr, ParserError> {
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

        self.expect(TokenKind::RBrace, "}")?;

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

        self.expect(TokenKind::RBrace, "}")?;
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
        self.expect(TokenKind::FatArrow, "map fat arrow `=>`")?;
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

        self.expect(closing, "literal terminator")?;
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

        self.expect(closing, expected_closing)?;
        Ok(items)
    }
}
