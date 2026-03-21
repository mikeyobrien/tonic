use super::*;
use crate::lexer::TokenKind;

impl<'a> Parser<'a> {
    pub(super) fn parse_anonymous_function_expression(&mut self) -> Result<Expr, ParserError> {
        let fn_span = self.expect_token(TokenKind::Fn, "fn")?.span();
        let offset = fn_span.start();
        let mut clauses = Vec::new();
        let mut expected_arity = None;

        loop {
            let (clause_span, patterns, guard, body) = self.parse_anonymous_function_clause()?;
            let clause_arity = patterns.len();
            if let Some(arity) = expected_arity {
                if arity != clause_arity {
                    return Err(self.anonymous_function_clause_arity_mismatch_error(
                        clause_span,
                        arity,
                        clause_arity,
                    ));
                }
            } else {
                expected_arity = Some(clause_arity);
            }

            clauses.push((patterns, guard, body));

            if self.match_kind(TokenKind::Semicolon) {
                if self.check(TokenKind::End) {
                    break;
                }
                continue;
            }

            if self.check(TokenKind::End) {
                break;
            }

            if self.is_at_end() {
                return Err(self.missing_end_error("anonymous function", fn_span));
            }
        }

        self.expect_block_end("anonymous function", fn_span)?;
        self.lower_anonymous_function_clauses(offset, clauses)
    }

    fn parse_anonymous_function_clause(
        &mut self,
    ) -> Result<(Span, Vec<Pattern>, Option<Expr>, Expr), ParserError> {
        let clause_span = self
            .current()
            .expect("anonymous function clause should start with a token")
            .span();
        let mut patterns = Vec::new();

        if !self.check(TokenKind::Arrow) {
            loop {
                patterns.push(self.parse_pattern()?);
                if self.match_kind(TokenKind::Comma) {
                    continue;
                }
                break;
            }
        }

        let guard = if self.match_kind(TokenKind::When) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect_clause_arrow(
            "anonymous function clause",
            clause_span,
            "add '->' between the anonymous function parameters and clause body",
        )?;
        let body = self.parse_branch_body()?;
        Ok((clause_span, patterns, guard, body))
    }

    fn lower_anonymous_function_clauses(
        &mut self,
        offset: usize,
        clauses: Vec<(Vec<Pattern>, Option<Expr>, Expr)>,
    ) -> Result<Expr, ParserError> {
        let Some((patterns, guard, body)) = clauses.first() else {
            return Err(ParserError::at_current(
                "anonymous function requires at least one clause",
                self.current(),
            ));
        };

        if clauses.len() == 1
            && guard.is_none()
            && patterns
                .iter()
                .all(|pattern| matches!(pattern, Pattern::Bind { name } if name != "_"))
        {
            let params = patterns
                .iter()
                .map(|pattern| match pattern {
                    Pattern::Bind { name } => name.clone(),
                    _ => unreachable!("validated bind-only parameter list"),
                })
                .collect::<Vec<_>>();

            return Ok(Expr::anonymous_fn(
                self.node_ids.next_expr(),
                offset,
                params,
                body.clone(),
            ));
        }

        let arity = patterns.len();
        let params = (0..arity)
            .map(|index| format!("__arg{index}"))
            .collect::<Vec<_>>();

        let subject = match arity {
            0 => Expr::nil(self.node_ids.next_expr(), offset),
            1 => Expr::variable(self.node_ids.next_expr(), offset, params[0].clone()),
            _ => {
                let items = params
                    .iter()
                    .map(|name| Expr::variable(self.node_ids.next_expr(), offset, name.clone()))
                    .collect::<Vec<_>>();
                Expr::tuple(self.node_ids.next_expr(), offset, items)
            }
        };

        let branches = clauses
            .into_iter()
            .map(|(patterns, guard, body)| {
                let head = match arity {
                    0 => Pattern::Nil,
                    1 => patterns.into_iter().next().unwrap_or(Pattern::Wildcard),
                    _ => Pattern::Tuple { items: patterns },
                };
                CaseBranch::new(head, guard, body)
            })
            .collect::<Vec<_>>();

        let body = Expr::case(self.node_ids.next_expr(), offset, subject, branches);

        Ok(Expr::anonymous_fn(
            self.node_ids.next_expr(),
            offset,
            params,
            body,
        ))
    }

    pub(super) fn parse_capture_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::Ampersand, "&")?.span().start();
        self.expect(TokenKind::LParen, "(")?;

        if self.check(TokenKind::RParen) {
            return Err(self.empty_capture_expression_error(offset));
        }

        self.capture_param_max_stack.push(0);
        let body = self.parse_expression()?;
        let max_capture_index = self
            .capture_param_max_stack
            .pop()
            .expect("capture placeholder scope should exist");

        self.expect(TokenKind::RParen, ")")?;

        if max_capture_index == 0 {
            return Err(ParserError::at_current(
                "capture expression requires at least one placeholder",
                self.current(),
            ));
        }

        let params = (1..=max_capture_index)
            .map(|index| format!("__capture{index}"))
            .collect::<Vec<_>>();

        Ok(Expr::anonymous_fn(
            self.node_ids.next_expr(),
            offset,
            params,
            body,
        ))
    }

    pub(super) fn parse_named_capture_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::Ampersand, "&")?.span().start();
        let mut segments = vec![self.expect_ident("captured function name")?];

        while self.match_kind(TokenKind::Dot) {
            segments.push(self.expect_ident("captured module or function segment")?);
        }

        let target = segments.join(".");
        if !self.match_kind(TokenKind::Slash) {
            return Err(self.missing_named_capture_arity_error(offset, &target));
        }

        let arity = self
            .expect_token(TokenKind::Integer, "function capture arity")?
            .lexeme()
            .parse::<usize>()
            .map_err(|_| {
                ParserError::at_current(
                    "function capture arity must be a positive integer",
                    self.current(),
                )
            })?;

        if arity == 0 {
            return Err(ParserError::at_current(
                "function capture arity must be >= 1",
                self.current(),
            ));
        }

        let callee = if segments.len() == 1 {
            segments.pop().expect("single segment should exist")
        } else {
            let function = segments.pop().expect("function segment should exist");
            format!("{}.{}", segments.join("."), function)
        };

        let params = (1..=arity)
            .map(|index| format!("__capture{index}"))
            .collect::<Vec<_>>();
        let args = params
            .iter()
            .map(|name| Expr::variable(self.node_ids.next_expr(), offset, name.clone()))
            .collect::<Vec<_>>();
        let body = Expr::call(self.node_ids.next_expr(), offset, callee, args);

        Ok(Expr::anonymous_fn(
            self.node_ids.next_expr(),
            offset,
            params,
            body,
        ))
    }
    pub(super) fn parse_ident_expression(&mut self) -> Result<Expr, ParserError> {
        let callee_token = self
            .advance()
            .expect("identifier token should be available");
        let offset = callee_token.span().start();
        let mut callee = callee_token.lexeme().to_string();
        let mut callee_end = callee_token.span().end();

        let has_module_qualifier = callee
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase());

        if self.check(TokenKind::Dot)
            && self
                .peek(1)
                .is_some_and(|token| token.kind() == TokenKind::Ident)
        {
            // Determine if this is a qualified call: Mod.func() or Outer.Inner.func().
            // Look ahead past any chain of .Ident segments to find a (.
            let is_qualified_call = 'qualified: {
                let mut ahead = 2usize; // after first Dot and Ident
                loop {
                    match self.peek(ahead).map(|t| t.kind()) {
                        Some(TokenKind::LParen) => break 'qualified true,
                        Some(TokenKind::Dot)
                            if self
                                .peek(ahead + 1)
                                .is_some_and(|t| t.kind() == TokenKind::Ident) =>
                        {
                            ahead += 2;
                        }
                        _ => break 'qualified false,
                    }
                }
            };

            // Also check no-paren call: Mod.func arg (only for single-segment qualifier)
            let is_no_paren_qualified = has_module_qualifier
                && self.peek(2).is_some_and(|token| {
                    token_can_start_no_paren_arg(token.kind())
                        && self.peek(1).is_some_and(|function| {
                            token.span().start() == function.span().end() + 1
                        })
                });

            if is_qualified_call || is_no_paren_qualified {
                // Consume all .Ident segments up to (but not including) the last ident,
                // building the full dotted callee.
                loop {
                    // peek: should be Dot
                    if !self.check(TokenKind::Dot) {
                        break;
                    }
                    // peek ahead 1: should be Ident
                    let next_is_ident = self.peek(1).is_some_and(|t| t.kind() == TokenKind::Ident);
                    if !next_is_ident {
                        break;
                    }
                    // peek ahead 2: determine if we should consume this segment
                    let after_next = self.peek(2).map(|t| t.kind());
                    // If after .Ident we have (, another .Ident, or a no-paren arg start, consume.
                    let should_consume = matches!(after_next, Some(TokenKind::LParen))
                        || matches!(after_next, Some(TokenKind::Dot))
                            && self.peek(3).is_some_and(|t| t.kind() == TokenKind::Ident)
                        || (is_no_paren_qualified
                            && after_next.is_some_and(token_can_start_no_paren_arg));
                    if !should_consume {
                        break;
                    }
                    // Consume the Dot
                    self.advance();
                    // Consume the Ident segment
                    let seg_token =
                        self.expect_token(TokenKind::Ident, "qualified name segment")?;
                    callee_end = seg_token.span().end();
                    callee = format!("{callee}.{}", seg_token.lexeme());
                }
            }
        }

        if self.match_kind(TokenKind::LParen) {
            let args = self.parse_call_args()?;
            self.expect(TokenKind::RParen, ")")?;
            return Ok(Expr::call(self.node_ids.next_expr(), offset, callee, args));
        }

        if self.current_starts_no_paren_call_arg(callee_end) {
            let args = self.parse_no_paren_call_args()?;
            return Ok(Expr::call(self.node_ids.next_expr(), offset, callee, args));
        }

        Ok(Expr::variable(self.node_ids.next_expr(), offset, callee))
    }
}
