use super::*;
use crate::lexer::TokenKind;

impl<'a> Parser<'a> {
    pub(super) fn parse_try_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::Try, "try")?.span().start();
        self.expect(TokenKind::Do, "do")?;
        let body = self.parse_block_body()?;

        let mut rescue = Vec::new();
        if self.match_kind(TokenKind::Rescue) {
            while !self.check(TokenKind::Catch)
                && !self.check(TokenKind::After)
                && !self.check(TokenKind::End)
            {
                if self.is_at_end() {
                    return Err(self.expected("rescue branch, catch branch, after block, or end"));
                }
                rescue.push(self.parse_rescue_branch()?);
            }
        }

        let mut catch = Vec::new();
        if self.match_kind(TokenKind::Catch) {
            while !self.check(TokenKind::After) && !self.check(TokenKind::End) {
                if self.is_at_end() {
                    return Err(self.expected("catch branch, after block, or end"));
                }
                catch.push(self.parse_case_branch()?);
            }
        }

        let after = if self.match_kind(TokenKind::After) {
            Some(self.parse_block_body()?)
        } else {
            None
        };

        if rescue.is_empty() && catch.is_empty() && after.is_none() {
            return Err(ParserError::at_current(
                "try must be followed by rescue, catch, or after",
                Some(&self.tokens[self.index - 1]),
            ));
        }

        self.expect(TokenKind::End, "end")?;

        Ok(Expr::try_expr(
            self.node_ids.next_expr(),
            offset,
            body,
            rescue,
            catch,
            after,
        ))
    }

    fn parse_rescue_branch(&mut self) -> Result<CaseBranch, ParserError> {
        if self.check(TokenKind::Ident)
            && self
                .peek(1)
                .is_some_and(|token| token.kind() == TokenKind::In)
        {
            let binding = self.expect_ident("rescue exception binding")?;
            self.expect(TokenKind::In, "in")?;
            let (module, module_offset) = self.parse_rescue_module_reference()?;
            let guard = self.parse_rescue_module_guard(binding.as_str(), module, module_offset)?;
            self.expect(TokenKind::Arrow, "->")?;
            let body = self.parse_branch_body()?;
            return Ok(CaseBranch::new(
                Pattern::Bind { name: binding },
                guard,
                body,
            ));
        }

        if self.current_starts_module_reference() {
            let (module, module_offset) = self.parse_rescue_module_reference()?;
            let binding = RESCUE_EXCEPTION_BINDING.to_string();
            let guard = self.parse_rescue_module_guard(binding.as_str(), module, module_offset)?;
            self.expect(TokenKind::Arrow, "->")?;
            let body = self.parse_branch_body()?;
            return Ok(CaseBranch::new(
                Pattern::Bind { name: binding },
                guard,
                body,
            ));
        }

        self.parse_case_branch()
    }

    fn parse_rescue_module_reference(&mut self) -> Result<(String, usize), ParserError> {
        let Some(current) = self.current() else {
            return Err(self.expected("rescue exception module"));
        };

        if current.kind() != TokenKind::Ident || !starts_with_uppercase(current.lexeme()) {
            return Err(ParserError::at_current(
                "rescue module match expects module reference starting with uppercase identifier",
                Some(current),
            ));
        }

        let offset = current.span().start();
        let module = self.parse_module_reference("rescue exception module")?;
        Ok((module, offset))
    }

    fn parse_rescue_module_guard(
        &mut self,
        binding: &str,
        module: String,
        offset: usize,
    ) -> Result<Option<Expr>, ParserError> {
        let module_guard = self.build_rescue_module_guard(binding, module, offset);
        let user_guard = if self.match_kind(TokenKind::When) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Some(if let Some(user_guard) = user_guard {
            Expr::binary(
                self.node_ids.next_expr(),
                BinaryOp::And,
                module_guard,
                user_guard,
            )
        } else {
            module_guard
        }))
    }

    fn build_rescue_module_guard(&mut self, binding: &str, module: String, offset: usize) -> Expr {
        let module_pattern = Pattern::Map {
            entries: vec![MapPatternEntry::new(
                Pattern::Atom {
                    value: "__exception__".to_string(),
                },
                Pattern::Atom { value: module },
            )],
        };

        Expr::case(
            self.node_ids.next_expr(),
            offset,
            Expr::variable(self.node_ids.next_expr(), offset, binding.to_string()),
            vec![
                CaseBranch::new(
                    module_pattern,
                    None,
                    Expr::bool(self.node_ids.next_expr(), offset, true),
                ),
                CaseBranch::new(
                    Pattern::Wildcard,
                    None,
                    Expr::bool(self.node_ids.next_expr(), offset, false),
                ),
            ],
        )
    }

    pub(super) fn parse_raise_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::Raise, "raise")?.span().start();

        let has_parens = self.match_kind(TokenKind::LParen);
        let error = if self.current_starts_module_reference() {
            let module_offset = self
                .current()
                .map(|token| token.span().start())
                .unwrap_or(offset);
            let module = self.parse_module_reference("exception module")?;
            let options = if self.match_kind(TokenKind::Comma) {
                self.parse_raise_keyword_entries()?
            } else {
                Vec::new()
            };
            self.build_structured_exception_expr(module, module_offset, options)
        } else {
            let error = self.parse_expression()?;
            if self.match_kind(TokenKind::Comma) {
                return Err(ParserError::at_current(
                    "structured raise expects module reference before keyword arguments",
                    self.current(),
                ));
            }
            error
        };

        if has_parens {
            self.expect(TokenKind::RParen, ")")?;
        }

        Ok(Expr::raise(self.node_ids.next_expr(), offset, error))
    }

    fn parse_raise_keyword_entries(&mut self) -> Result<Vec<LabelExprEntry>, ParserError> {
        if !self.starts_keyword_literal_entry() {
            return Err(ParserError::at_current(
                "structured raise expects keyword arguments after module",
                self.current(),
            ));
        }

        let mut entries = Vec::new();
        loop {
            let key = self.expect_ident("raise option key")?;
            self.expect(TokenKind::Colon, ":")?;
            let value = self.parse_expression()?;
            entries.push(LabelExprEntry { key, value });

            if self.match_kind(TokenKind::Comma) {
                if self.check(TokenKind::RParen) {
                    return Err(ParserError::at_current(
                        "structured raise expects keyword arguments after module",
                        self.current(),
                    ));
                }
                continue;
            }

            break;
        }

        Ok(entries)
    }

    fn build_structured_exception_expr(
        &mut self,
        module: String,
        offset: usize,
        options: Vec<LabelExprEntry>,
    ) -> Expr {
        let mut message_expr = Expr::string(
            self.node_ids.next_expr(),
            offset,
            "exception raised".to_string(),
        );
        let mut metadata_entries = Vec::new();

        for option in options {
            if option.key == "message" {
                message_expr = option.value;
            } else {
                metadata_entries.push(MapExprEntry {
                    key: Expr::atom(self.node_ids.next_expr(), offset, option.key),
                    value: option.value,
                });
            }
        }

        let metadata_expr = Expr::map(self.node_ids.next_expr(), offset, metadata_entries);
        let entries = vec![
            MapExprEntry {
                key: Expr::atom(
                    self.node_ids.next_expr(),
                    offset,
                    "__exception__".to_string(),
                ),
                value: Expr::atom(self.node_ids.next_expr(), offset, module),
            },
            MapExprEntry {
                key: Expr::atom(self.node_ids.next_expr(), offset, "message".to_string()),
                value: message_expr,
            },
            MapExprEntry {
                key: Expr::atom(self.node_ids.next_expr(), offset, "metadata".to_string()),
                value: metadata_expr,
            },
        ];

        Expr::map(self.node_ids.next_expr(), offset, entries)
    }
}
