use super::*;
use crate::lexer::TokenKind;

impl<'a> Parser<'a> {
    pub(super) fn parse_pattern(&mut self) -> Result<Pattern, ParserError> {
        // Bitstring pattern: <<p1, p2, ...>>
        if self.check(TokenKind::LtLt) {
            let opening_span = self
                .advance()
                .expect("bitstring pattern opener should be available")
                .span();
            let mut items = Vec::new();
            // Check for empty <<>>
            if !self.check(TokenKind::GtGt) {
                loop {
                    items.push(self.parse_pattern()?);
                    if self.match_kind(TokenKind::Comma) {
                        continue;
                    }
                    if !self.check(TokenKind::GtGt)
                        && self.current_starts_missing_bitstring_pattern_comma()
                    {
                        return Err(self.missing_comma_error(
                            "bitstring pattern",
                            "separate bitstring pattern elements with commas, for example `<<left, right>>`",
                        ));
                    }
                    break;
                }
            }
            self.expect_closing_delimiter(
                TokenKind::GtGt,
                ">>",
                "bitstring pattern",
                opening_span,
                "add '>>' to close the bitstring pattern, for example `<<left, right>> -> ...`",
            )?;
            return Ok(Pattern::Bitstring { items });
        }

        if self.match_kind(TokenKind::Caret) {
            let name = self.expect_ident("pinned variable")?;
            return Ok(Pattern::Pin { name });
        }

        if self.check(TokenKind::Integer) {
            let token = self.advance().expect("integer token should be available");
            let value = token.lexeme().parse::<i64>().map_err(|_| {
                ParserError::at_current(
                    format!("invalid integer literal '{}'", token.lexeme()),
                    Some(token),
                )
            })?;
            return Ok(Pattern::Integer { value });
        }

        if self.check(TokenKind::True) {
            self.advance();
            return Ok(Pattern::Bool { value: true });
        }

        if self.check(TokenKind::False) {
            self.advance();
            return Ok(Pattern::Bool { value: false });
        }

        if self.check(TokenKind::Nil) {
            self.advance();
            return Ok(Pattern::Nil);
        }

        if self.check(TokenKind::String) {
            let value = self
                .advance()
                .expect("string token should be available")
                .lexeme()
                .to_string();
            return Ok(Pattern::String { value });
        }

        if self.match_kind(TokenKind::LBrace) {
            let (items, tail) = self.parse_pattern_items(TokenKind::RBrace)?;
            if tail.is_some() {
                return Err(ParserError::at_current(
                    "tuple patterns do not support tail syntax",
                    self.current(),
                ));
            }
            return Ok(Pattern::Tuple { items });
        }

        if self.match_kind(TokenKind::LBracket) {
            let (items, tail) = self.parse_pattern_items(TokenKind::RBracket)?;
            return Ok(Pattern::List { items, tail });
        }

        if self.match_kind(TokenKind::Percent) {
            return self.parse_percent_pattern();
        }

        if self.check(TokenKind::Atom) {
            let value = self
                .advance()
                .expect("atom token should be available")
                .lexeme()
                .to_string();
            return Ok(Pattern::Atom { value });
        }

        if self.check(TokenKind::Ident) {
            let name = self
                .advance()
                .expect("identifier token should be available")
                .lexeme()
                .to_string();

            if name == "_" {
                return Ok(Pattern::Wildcard);
            }

            return Ok(Pattern::Bind { name });
        }

        Err(self.expected("pattern"))
    }

    fn parse_pattern_items(
        &mut self,
        closing: TokenKind,
    ) -> Result<(Vec<Pattern>, Option<Box<Pattern>>), ParserError> {
        let mut items = Vec::new();
        let mut tail = None;

        if self.check(closing) {
            self.advance();
            return Ok((items, tail));
        }

        loop {
            items.push(self.parse_pattern()?);

            if self.match_kind(TokenKind::Pipe) {
                tail = Some(Box::new(self.parse_pattern()?));
                break;
            }

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        self.expect(closing, "pattern terminator")?;
        Ok((items, tail))
    }

    fn parse_percent_pattern(&mut self) -> Result<Pattern, ParserError> {
        if self.check(TokenKind::LBrace) {
            return self.parse_map_pattern();
        }

        self.parse_struct_pattern()
    }

    fn parse_map_pattern(&mut self) -> Result<Pattern, ParserError> {
        self.expect(TokenKind::LBrace, "{")?;

        let mut entries = Vec::new();
        if !self.check(TokenKind::RBrace) {
            loop {
                let (key, value) = if self.check(TokenKind::Ident)
                    && self
                        .peek(1)
                        .is_some_and(|token| token.kind() == TokenKind::Colon)
                {
                    let key = Pattern::Atom {
                        value: self.expect_ident("map pattern key")?,
                    };
                    self.expect(TokenKind::Colon, ":")?;
                    let value = self.parse_pattern()?;
                    (key, value)
                } else {
                    let key = self.parse_pattern()?;
                    if !(self.match_kind(TokenKind::FatArrow) || self.match_kind(TokenKind::Arrow))
                    {
                        return Err(self.missing_map_fat_arrow_error(
                            "map pattern entry",
                            "write `%{key => pattern}` for computed keys, or use `%{name: pattern}` when the key is an atom label",
                        ));
                    }
                    let value = self.parse_pattern()?;
                    (key, value)
                };

                entries.push(MapPatternEntry::new(key, value));

                if self.match_kind(TokenKind::Comma) {
                    continue;
                }

                break;
            }
        }

        self.expect(TokenKind::RBrace, "}")?;

        Ok(Pattern::Map { entries })
    }

    fn parse_struct_pattern(&mut self) -> Result<Pattern, ParserError> {
        let module = self.parse_module_reference("struct module")?;
        self.expect(TokenKind::LBrace, "{")?;

        let mut entries = Vec::new();
        if !self.check(TokenKind::RBrace) {
            loop {
                let key = self.expect_ident("struct pattern key")?;
                self.expect(TokenKind::Colon, ":")?;
                let value = self.parse_pattern()?;
                entries.push(LabelPatternEntry { key, value });

                if self.match_kind(TokenKind::Comma) {
                    continue;
                }

                break;
            }
        }

        self.expect(TokenKind::RBrace, "}")?;

        Ok(Pattern::Struct { module, entries })
    }
}
