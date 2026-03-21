use super::*;
use crate::lexer::TokenKind;

impl<'a> Parser<'a> {
    pub(super) fn parse_if_expression(&mut self) -> Result<Expr, ParserError> {
        let if_span = self.expect_token(TokenKind::If, "if")?.span();
        let offset = if_span.start();
        let condition = self.parse_expression()?;
        self.expect_block_do(
            "if expression",
            if_span,
            "add 'do' after the if condition to begin the then branch",
        )?;

        let then_body = self.parse_block_body()?;
        let else_body = if self.match_kind(TokenKind::Else) {
            self.parse_block_body()?
        } else {
            Expr::nil(self.node_ids.next_expr(), offset)
        };

        self.expect_block_end("if expression", if_span)?;

        Ok(self.lower_guarded_control_case(offset, condition, then_body, else_body))
    }

    pub(super) fn parse_unless_expression(&mut self) -> Result<Expr, ParserError> {
        let unless_span = self.expect_token(TokenKind::Unless, "unless")?.span();
        let offset = unless_span.start();
        let condition = self.parse_expression()?;
        self.expect_block_do(
            "unless expression",
            unless_span,
            "add 'do' after the unless condition to begin the then branch",
        )?;

        let then_body = self.parse_block_body()?;
        let else_body = if self.match_kind(TokenKind::Else) {
            self.parse_block_body()?
        } else {
            Expr::nil(self.node_ids.next_expr(), offset)
        };

        self.expect_block_end("unless expression", unless_span)?;

        Ok(self.lower_guarded_control_case(offset, condition, else_body, then_body))
    }

    pub(super) fn parse_cond_expression(&mut self) -> Result<Expr, ParserError> {
        let cond_span = self.expect_token(TokenKind::Cond, "cond")?.span();
        let offset = cond_span.start();
        self.expect_block_do(
            "cond expression",
            cond_span,
            "add 'do' after 'cond' to begin its branches",
        )?;

        let mut branches = Vec::new();
        while !self.check(TokenKind::End) {
            if self.is_at_end() {
                return Err(self.missing_end_error("cond expression", cond_span));
            }

            let condition = self.parse_expression()?;
            self.expect(TokenKind::Arrow, "->")?;
            let body = self.parse_branch_body()?;
            let guard = self.lower_truthy_guard(condition);
            branches.push(CaseBranch::new(Pattern::Wildcard, Some(guard), body));
        }

        self.expect_block_end("cond expression", cond_span)?;

        Ok(Expr::case(
            self.node_ids.next_expr(),
            offset,
            Expr::nil(self.node_ids.next_expr(), offset),
            branches,
        ))
    }

    pub(super) fn parse_with_expression(&mut self) -> Result<Expr, ParserError> {
        let with_span = self.expect_token(TokenKind::With, "with")?.span();
        let offset = with_span.start();
        let mut clauses = Vec::new();

        loop {
            let pattern = self.parse_pattern()?;
            self.expect(TokenKind::LeftArrow, "<-")?;
            let value = self.parse_expression()?;
            clauses.push((pattern, value));

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        self.expect_block_do(
            "with expression",
            with_span,
            "add 'do' after the with clauses to begin the main body",
        )?;
        let body = self.parse_block_body()?;

        let else_branches = if self.match_kind(TokenKind::Else) {
            let mut branches = Vec::new();

            while !self.check(TokenKind::End) {
                if self.is_at_end() {
                    return Err(self.missing_end_error("with expression", with_span));
                }

                branches.push(self.parse_case_branch()?);
            }

            branches
        } else {
            Vec::new()
        };

        self.expect_block_end("with expression", with_span)?;

        Ok(self.lower_with_expression(offset, clauses, body, else_branches))
    }

    pub(super) fn parse_for_expression(&mut self) -> Result<Expr, ParserError> {
        let for_span = self.expect_token(TokenKind::For, "for")?.span();
        let offset = for_span.start();

        let mut generators = Vec::new();
        let mut into_expr = None;
        let mut reduce_expr = None;

        loop {
            if self.check(TokenKind::Ident)
                && self
                    .peek(1)
                    .is_some_and(|token| token.kind() == TokenKind::Colon)
            {
                let option_token = self.expect_token(TokenKind::Ident, "for option")?;
                self.expect(TokenKind::Colon, ":")?;

                match option_token.lexeme() {
                    "into" => {
                        if into_expr.is_some() {
                            return Err(ParserError::at_current(
                                "duplicate for option 'into'",
                                Some(option_token),
                            ));
                        }
                        into_expr = Some(self.parse_expression()?);
                    }
                    "reduce" => {
                        if reduce_expr.is_some() {
                            return Err(ParserError::at_current(
                                "duplicate for option 'reduce'",
                                Some(option_token),
                            ));
                        }
                        reduce_expr = Some(self.parse_expression()?);
                    }
                    other => {
                        return Err(ParserError::at_current(
                            format!(
                                "unsupported for option '{other}'; supported options are into and reduce"
                            ),
                            Some(option_token),
                        ));
                    }
                }

                if self.match_kind(TokenKind::Comma) {
                    continue;
                }
                break;
            }

            let pattern = self.parse_pattern()?;
            let guard = if self.match_kind(TokenKind::When) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            self.expect(TokenKind::LeftArrow, "<-")?;
            let generator = self.parse_expression()?;
            generators.push(ForGenerator::new(pattern, generator, guard));

            if self.match_kind(TokenKind::Comma) {
                continue;
            }
            break;
        }

        if generators.is_empty() {
            return Err(ParserError::at_current(
                "for expects at least one generator",
                self.current(),
            ));
        }

        if reduce_expr.is_some() && into_expr.is_some() {
            return Err(ParserError::at_current(
                "for options 'reduce' and 'into' cannot be combined",
                self.current(),
            ));
        }

        self.expect_block_do(
            "for expression",
            for_span,
            "add 'do' after the for clauses to begin the comprehension body",
        )?;
        let body = if reduce_expr.is_some() {
            self.parse_for_reduce_body(offset, for_span)?
        } else {
            self.parse_block_body()?
        };
        self.expect_block_end("for expression", for_span)?;

        Ok(Expr::for_comprehension(
            self.node_ids.next_expr(),
            offset,
            generators,
            into_expr,
            reduce_expr,
            body,
        ))
    }

    fn parse_for_reduce_body(
        &mut self,
        offset: usize,
        for_span: crate::lexer::Span,
    ) -> Result<Expr, ParserError> {
        let mut branches = Vec::new();

        while !self.check(TokenKind::End) {
            if self.is_at_end() {
                return Err(self.missing_end_error("for expression", for_span));
            }
            branches.push(self.parse_case_branch()?);
            if self.match_kind(TokenKind::Semicolon) {
                continue;
            }
        }

        if branches.is_empty() {
            return Err(ParserError::at_current(
                "for reduce expects at least one accumulator clause",
                self.current(),
            ));
        }

        Ok(Expr::case(
            self.node_ids.next_expr(),
            offset,
            Expr::variable(
                self.node_ids.next_expr(),
                offset,
                FOR_REDUCE_ACC_BINDING.to_string(),
            ),
            branches,
        ))
    }

    fn lower_with_expression(
        &mut self,
        offset: usize,
        clauses: Vec<(Pattern, Expr)>,
        body: Expr,
        else_branches: Vec<CaseBranch>,
    ) -> Expr {
        let mut lowered = body;

        for (pattern, value) in clauses.into_iter().rev() {
            let failure_binding = "__tonic_with_failure".to_string();
            let failure_handler = if else_branches.is_empty() {
                Expr::variable(self.node_ids.next_expr(), offset, failure_binding.clone())
            } else {
                Expr::case(
                    self.node_ids.next_expr(),
                    offset,
                    Expr::variable(self.node_ids.next_expr(), offset, failure_binding.clone()),
                    else_branches.clone(),
                )
            };

            lowered = Expr::case(
                self.node_ids.next_expr(),
                value.offset(),
                value,
                vec![
                    CaseBranch::new(pattern, None, lowered),
                    CaseBranch::new(
                        Pattern::Bind {
                            name: failure_binding,
                        },
                        None,
                        failure_handler,
                    ),
                ],
            );
        }

        lowered
    }

    pub(super) fn lower_guarded_control_case(
        &mut self,
        offset: usize,
        condition: Expr,
        truthy_body: Expr,
        fallback_body: Expr,
    ) -> Expr {
        let guard = self.lower_truthy_guard(condition);

        Expr::case(
            self.node_ids.next_expr(),
            offset,
            Expr::nil(self.node_ids.next_expr(), offset),
            vec![
                CaseBranch::new(Pattern::Wildcard, Some(guard), truthy_body),
                CaseBranch::new(Pattern::Wildcard, None, fallback_body),
            ],
        )
    }

    pub(super) fn lower_truthy_guard(&mut self, condition: Expr) -> Expr {
        let offset = condition.offset();
        let first_bang = Expr::unary(self.node_ids.next_expr(), offset, UnaryOp::Bang, condition);
        Expr::unary(self.node_ids.next_expr(), offset, UnaryOp::Bang, first_bang)
    }

    pub(super) fn parse_case_expression(&mut self) -> Result<Expr, ParserError> {
        let case_span = self.expect_token(TokenKind::Case, "case")?.span();
        let offset = case_span.start();
        let subject = self.parse_expression()?;
        self.expect_block_do(
            "case expression",
            case_span,
            "add 'do' after the case subject to begin the case branches",
        )?;

        let mut branches = Vec::new();
        while !self.check(TokenKind::End) {
            if self.is_at_end() {
                return Err(self.missing_end_error("case expression", case_span));
            }

            branches.push(self.parse_case_branch()?);
        }

        self.expect_block_end("case expression", case_span)?;

        Ok(Expr::case(
            self.node_ids.next_expr(),
            offset,
            subject,
            branches,
        ))
    }

    pub(super) fn parse_case_branch(&mut self) -> Result<CaseBranch, ParserError> {
        let pattern = self.parse_pattern()?;
        let guard = if self.match_kind(TokenKind::When) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect(TokenKind::Arrow, "->")?;
        let body = self.parse_branch_body()?;

        Ok(CaseBranch::new(pattern, guard, body))
    }
}
