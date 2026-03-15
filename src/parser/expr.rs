use super::*;
use crate::lexer::TokenKind;

impl<'a> Parser<'a> {
    pub(super) fn parse_expression(&mut self) -> Result<Expr, ParserError> {
        self.parse_match_expression()
    }

    /// Parse sequential expressions in a block body (between `do`/`end`).
    pub(super) fn parse_block_body(&mut self) -> Result<Expr, ParserError> {
        let offset = self.current().map(|t| t.span().start()).unwrap_or(0);
        let at_end = |s: &Self| {
            s.check(TokenKind::End)
                || s.check(TokenKind::Else)
                || s.check(TokenKind::Rescue)
                || s.check(TokenKind::Catch)
                || s.check(TokenKind::After)
                || s.is_at_end()
        };
        let first = self.parse_expression()?;
        if at_end(self) {
            return Ok(first);
        }
        let id = self.node_ids.next_expr();
        let mut exprs = vec![first];
        while !at_end(self) {
            exprs.push(self.parse_expression()?);
        }
        Ok(Expr::Block { id, offset, exprs })
    }

    fn parse_match_expression(&mut self) -> Result<Expr, ParserError> {
        let left = self.parse_pipe_expression()?;

        if self.match_kind(TokenKind::MatchEq) {
            let right = self.parse_match_expression()?;
            return Ok(Expr::binary(
                self.node_ids.next_expr(),
                BinaryOp::Match,
                left,
                right,
            ));
        }

        Ok(left)
    }

    fn parse_pipe_expression(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_binary_expression(0)?;

        while self.match_kind(TokenKind::PipeGt) {
            let right = self.parse_binary_expression(0)?;
            left = Expr::pipe(self.node_ids.next_expr(), left, right);
        }

        Ok(left)
    }

    fn parse_binary_expression(&mut self, min_precedence: u8) -> Result<Expr, ParserError> {
        let mut left = self.parse_unary_expression()?;

        while let Some((precedence, next_precedence, op)) = self.current_binary_operator() {
            if precedence < min_precedence {
                break;
            }

            self.advance();

            // `not in` consumes two tokens (not + in)
            if op == BinaryOp::NotIn {
                self.advance(); // consume the `in` token
            }

            let right = self.parse_binary_expression(next_precedence)?;

            // Handle stepped range: after parsing `a..b`, check for `//step`
            let left_with_op = if op == BinaryOp::Range && self.check(TokenKind::SlashSlash) {
                self.advance(); // consume `//`
                let step = self.parse_binary_expression(next_precedence)?;
                // Encode as SteppedRange with left=Expr::binary(Range, left, right), step=step
                // We wrap it as Binary(SteppedRange, Binary(Range, left, right), step)
                let range_node =
                    Expr::binary(self.node_ids.next_expr(), BinaryOp::Range, left, right);
                Expr::binary(
                    self.node_ids.next_expr(),
                    BinaryOp::SteppedRange,
                    range_node,
                    step,
                )
            } else {
                Expr::binary(self.node_ids.next_expr(), op, left, right)
            };

            left = left_with_op;
        }

        Ok(left)
    }

    fn parse_unary_expression(&mut self) -> Result<Expr, ParserError> {
        if let Some(token) = self.current() {
            let unary = match token.kind() {
                TokenKind::Not => {
                    // Only treat `not` as unary if NOT followed by `in` (which would be `not in`)
                    if self.peek(1).map(|t| t.kind()) == Some(TokenKind::In) {
                        None
                    } else {
                        Some((UnaryOp::Not, 110))
                    }
                }
                TokenKind::Bang => Some((UnaryOp::Bang, 110)),
                TokenKind::Plus => Some((UnaryOp::Plus, 110)),
                TokenKind::Minus => Some((UnaryOp::Minus, 110)),
                TokenKind::TildeTildeTilde => Some((UnaryOp::BitwiseNot, 110)),
                _ => None,
            };

            if let Some((op, rbp)) = unary {
                let offset = self.advance().unwrap().span().start();
                let expr = self.parse_binary_expression(rbp)?;
                return Ok(Expr::unary(self.node_ids.next_expr(), offset, op, expr));
            }
        }

        self.parse_postfix_expression()
    }

    pub(super) fn parse_postfix_expression(&mut self) -> Result<Expr, ParserError> {
        let mut expression = self.parse_atomic_expression()?;

        loop {
            if self.check(TokenKind::Question) {
                let offset = self
                    .advance()
                    .expect("question token should be available")
                    .span()
                    .start();
                expression = Expr::question(self.node_ids.next_expr(), offset, expression);
                continue;
            }

            if self.check(TokenKind::Dot) {
                if self
                    .peek(1)
                    .is_some_and(|token| token.kind() == TokenKind::LParen)
                {
                    let offset = self
                        .advance()
                        .expect("dot token should be available")
                        .span()
                        .start();
                    self.expect(TokenKind::LParen, "(")?;
                    let args = self.parse_call_args()?;
                    self.expect(TokenKind::RParen, ")")?;
                    expression = Expr::invoke(self.node_ids.next_expr(), offset, expression, args);
                    continue;
                } else if self
                    .peek(1)
                    .is_some_and(|token| token.kind() == TokenKind::Ident)
                {
                    let offset = self
                        .advance()
                        .expect("dot token should be available")
                        .span()
                        .start();
                    let label = self.expect_ident("field access label")?;
                    expression =
                        Expr::field_access(self.node_ids.next_expr(), offset, expression, label);
                    continue;
                }
            }

            if self.check(TokenKind::LBracket) {
                let has_space_before = self.index > 0
                    && self.tokens[self.index - 1].span().end()
                        < self.current().unwrap().span().start();

                if has_space_before {
                    break;
                }

                let offset = self
                    .advance()
                    .expect("lbracket token should be available")
                    .span()
                    .start();
                let index = self.parse_expression()?;
                self.expect(TokenKind::RBracket, "]")?;
                expression =
                    Expr::index_access(self.node_ids.next_expr(), offset, expression, index);
                continue;
            }

            break;
        }

        Ok(expression)
    }

    pub(super) fn parse_atomic_expression(&mut self) -> Result<Expr, ParserError> {
        if self.check(TokenKind::LtLt) {
            return self.parse_bitstring_literal_expression();
        }

        if self.check(TokenKind::If) {
            return self.parse_if_expression();
        }

        if self.check(TokenKind::Unless) {
            return self.parse_unless_expression();
        }

        if self.check(TokenKind::Cond) {
            return self.parse_cond_expression();
        }

        if self.check(TokenKind::With) {
            return self.parse_with_expression();
        }

        if self.check(TokenKind::For) {
            return self.parse_for_expression();
        }

        if self.check(TokenKind::Case) {
            return self.parse_case_expression();
        }

        if self.check(TokenKind::Try) {
            return self.parse_try_expression();
        }

        if self.check(TokenKind::Raise) {
            return self.parse_raise_expression();
        }

        if self.check(TokenKind::Fn) {
            return self.parse_anonymous_function_expression();
        }

        if self.check(TokenKind::Ampersand) {
            if self
                .peek(1)
                .is_some_and(|token| token.kind() == TokenKind::LParen)
            {
                return self.parse_capture_expression();
            }

            if self
                .peek(1)
                .is_some_and(|token| token.kind() == TokenKind::Ident)
            {
                return self.parse_named_capture_expression();
            }

            if self
                .peek(1)
                .is_some_and(|token| token.kind() == TokenKind::Integer)
            {
                let offset = self
                    .advance()
                    .expect("ampersand token should be available")
                    .span()
                    .start();
                let placeholder = self
                    .expect_token(TokenKind::Integer, "capture placeholder index")?
                    .lexeme()
                    .parse::<usize>()
                    .map_err(|_| {
                        ParserError::at_current(
                            "capture placeholder index must be a positive integer",
                            self.current(),
                        )
                    })?;

                if placeholder == 0 {
                    return Err(ParserError::at_current(
                        "capture placeholder index must be >= 1",
                        self.current(),
                    ));
                }

                if let Some(current_max) = self.capture_param_max_stack.last_mut() {
                    *current_max = (*current_max).max(placeholder);
                } else {
                    return Err(ParserError::at_current(
                        "capture placeholders are only valid inside capture expressions",
                        self.current(),
                    ));
                }

                return Ok(Expr::variable(
                    self.node_ids.next_expr(),
                    offset,
                    format!("__capture{placeholder}"),
                ));
            }

            return Err(ParserError::at_current(
                "unsupported capture expression form; expected &(expr), &1, or &Module.fun/arity",
                self.current(),
            ));
        }

        if self.check(TokenKind::True) {
            let token = self.advance().expect("true token should be available");
            return Ok(Expr::bool(
                self.node_ids.next_expr(),
                token.span().start(),
                true,
            ));
        }

        if self.check(TokenKind::False) {
            let token = self.advance().expect("false token should be available");
            return Ok(Expr::bool(
                self.node_ids.next_expr(),
                token.span().start(),
                false,
            ));
        }

        if self.check(TokenKind::Nil) {
            let token = self.advance().expect("nil token should be available");
            return Ok(Expr::nil(self.node_ids.next_expr(), token.span().start()));
        }

        if self.check(TokenKind::String) {
            let token = self.advance().expect("string token should be available");
            let offset = token.span().start();
            let value = token.lexeme().to_string();
            return Ok(Expr::string(self.node_ids.next_expr(), offset, value));
        }

        if self.check(TokenKind::StringStart) {
            return self.parse_interpolated_string_expression();
        }

        if self.check(TokenKind::Float) {
            let token = self.advance().expect("float token should be available");
            let offset = token.span().start();
            let value = token.lexeme().to_string();
            return Ok(Expr::float(self.node_ids.next_expr(), offset, value));
        }

        if self.check(TokenKind::Integer) {
            let token = self.advance().expect("integer token should be available");
            let offset = token.span().start();
            let value = token.lexeme().parse::<i64>().map_err(|_| {
                ParserError::at_current(
                    format!("invalid integer literal '{}'", token.lexeme()),
                    Some(token),
                )
            })?;

            return Ok(Expr::int(self.node_ids.next_expr(), offset, value));
        }

        if self.check(TokenKind::Atom) {
            let token = self.advance().expect("atom token should be available");
            let offset = token.span().start();
            let value = token.lexeme().to_string();
            return Ok(Expr::atom(self.node_ids.next_expr(), offset, value));
        }

        if self.check(TokenKind::LBrace) {
            return self.parse_tuple_literal_expression();
        }

        if self.check(TokenKind::LBracket) {
            return self.parse_list_or_keyword_literal_expression();
        }

        if self.check(TokenKind::Percent) {
            return self.parse_percent_expression();
        }

        // Handle @attr_name in expression position (module attribute reference)
        if self.check(TokenKind::At) {
            let at_token = self.advance().expect("@ token should be available");
            let offset = at_token.span().start();
            let attr_name = self.expect_ident("attribute name")?;
            return Ok(Expr::variable(
                self.node_ids.next_expr(),
                offset,
                format!("@{attr_name}"),
            ));
        }

        if self.check(TokenKind::Ident) {
            return self.parse_ident_expression();
        }

        // Handle parenthesized expressions: (expr)
        if self.check(TokenKind::LParen) {
            let offset = self
                .advance()
                .expect("lparen token should be available")
                .span()
                .start();
            let inner = self.parse_expression()?;
            self.expect(TokenKind::RParen, ")")?;
            return Ok(Expr::group(self.node_ids.next_expr(), offset, inner));
        }

        Err(self.expected("expression"))
    }

    pub(super) fn parse_call_args(&mut self) -> Result<Vec<Expr>, ParserError> {
        let mut args = Vec::new();

        if self.check(TokenKind::RParen) {
            return Ok(args);
        }

        loop {
            args.push(self.parse_expression()?);

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        Ok(args)
    }

    pub(super) fn parse_no_paren_call_args(&mut self) -> Result<Vec<Expr>, ParserError> {
        let mut args = Vec::new();

        loop {
            args.push(self.parse_expression()?);

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        Ok(args)
    }

    pub(super) fn current_binary_operator(&self) -> Option<(u8, u8, BinaryOp)> {
        self.current().and_then(|token| match token.kind() {
            TokenKind::Star => Some((100, 101, BinaryOp::Mul)),
            TokenKind::Slash => Some((100, 101, BinaryOp::Div)),
            TokenKind::Ident if token.lexeme() == "div" => Some((100, 101, BinaryOp::IntDiv)),
            TokenKind::Ident if token.lexeme() == "rem" => Some((100, 101, BinaryOp::Rem)),
            TokenKind::Plus => Some((90, 91, BinaryOp::Plus)),
            TokenKind::Minus => Some((90, 91, BinaryOp::Minus)),
            TokenKind::LessGreater => Some((80, 80, BinaryOp::Concat)),
            TokenKind::PlusPlus => Some((80, 80, BinaryOp::PlusPlus)),
            TokenKind::MinusMinus => Some((80, 80, BinaryOp::MinusMinus)),
            TokenKind::DotDot => Some((80, 80, BinaryOp::Range)),
            TokenKind::In => Some((70, 71, BinaryOp::In)),
            TokenKind::Not => {
                // `not in` is a binary operator
                if self.peek(1).map(|t| t.kind()) == Some(TokenKind::In) {
                    Some((70, 71, BinaryOp::NotIn))
                } else {
                    None
                }
            }
            TokenKind::StrictEq => Some((60, 61, BinaryOp::StrictEq)),
            TokenKind::StrictBangEq => Some((60, 61, BinaryOp::StrictBangEq)),
            TokenKind::EqEq => Some((60, 61, BinaryOp::Eq)),
            TokenKind::BangEq => Some((60, 61, BinaryOp::NotEq)),
            TokenKind::Lt => Some((60, 61, BinaryOp::Lt)),
            TokenKind::LtEq => Some((60, 61, BinaryOp::Lte)),
            TokenKind::Gt => Some((60, 61, BinaryOp::Gt)),
            TokenKind::GtEq => Some((60, 61, BinaryOp::Gte)),
            TokenKind::AmpAmpAmp => Some((75, 76, BinaryOp::BitwiseAnd)),
            TokenKind::PipePipePipe => Some((73, 74, BinaryOp::BitwiseOr)),
            TokenKind::CaretCaretCaret => Some((74, 75, BinaryOp::BitwiseXor)),
            TokenKind::LtLtLt => Some((77, 78, BinaryOp::BitwiseShiftLeft)),
            TokenKind::GtGtGt => Some((77, 78, BinaryOp::BitwiseShiftRight)),
            TokenKind::AndAnd => Some((50, 51, BinaryOp::AndAnd)),
            TokenKind::And => Some((50, 51, BinaryOp::And)),
            TokenKind::OrOr => Some((40, 41, BinaryOp::OrOr)),
            TokenKind::Or => Some((40, 41, BinaryOp::Or)),
            _ => None,
        })
    }

    pub(super) fn current_starts_no_paren_call_arg(&self, callee_end: usize) -> bool {
        let Some(current) = self.current() else {
            return false;
        };

        if current.span().start() != callee_end + 1 {
            return false;
        }

        if current.kind() == TokenKind::Ident && current.lexeme() == "_" {
            return false;
        }

        token_can_start_no_paren_arg(current.kind())
    }
}
