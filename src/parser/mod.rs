use crate::lexer::{Span, Token, TokenKind};

mod ast;
mod canonicalize;
mod control;
mod expr;
mod fn_expr;
mod imports;
mod literal;
mod module;
mod pattern;
mod try_expr;

pub use ast::*;

pub(crate) const FOR_REDUCE_ACC_BINDING: &str = "__tonic_for_acc";
pub(crate) const RESCUE_EXCEPTION_BINDING: &str = "__tonic_rescue_exception";

pub(crate) fn starts_with_uppercase(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
}

pub(crate) fn is_builtin_call_target(callee: &str) -> bool {
    use crate::guard_builtins;
    matches!(
        callee,
        "ok" | "err"
            | "tuple"
            | "list"
            | "map"
            | "keyword"
            | "protocol_dispatch"
            | "host_call"
            | "abs"
            | "length"
            | "hd"
            | "tl"
            | "elem"
            | "tuple_size"
            | "to_string"
            | "max"
            | "min"
            | "round"
            | "trunc"
            | "map_size"
            | "put_elem"
            | "inspect"
    ) || guard_builtins::is_guard_builtin(callee)
}

pub(crate) fn token_can_start_no_paren_arg(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Ident
            | TokenKind::Atom
            | TokenKind::Integer
            | TokenKind::Float
            | TokenKind::String
            | TokenKind::StringStart
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Nil
            | TokenKind::LParen
            | TokenKind::LBrace
            | TokenKind::LBracket
            | TokenKind::Percent
            | TokenKind::Fn
            | TokenKind::If
            | TokenKind::Unless
            | TokenKind::Case
            | TokenKind::Cond
            | TokenKind::With
            | TokenKind::For
            | TokenKind::Try
            | TokenKind::Raise
            | TokenKind::Ampersand
    )
}

pub(crate) fn token_can_start_pattern(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Ident
            | TokenKind::Atom
            | TokenKind::Integer
            | TokenKind::String
            | TokenKind::LBrace
            | TokenKind::LBracket
            | TokenKind::Percent
            | TokenKind::Caret
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Nil
            | TokenKind::LtLt
    )
}

pub fn parse_ast(tokens: &[Token]) -> Result<Ast, ParserError> {
    Parser::new(tokens).parse_program()
}

pub(crate) struct Parser<'a> {
    pub(crate) tokens: &'a [Token],
    pub(crate) index: usize,
    pub(crate) node_ids: NodeIdGenerator,
    pub(crate) capture_param_max_stack: Vec<usize>,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            index: 0,
            node_ids: NodeIdGenerator::default(),
            capture_param_max_stack: Vec::new(),
        }
    }

    fn parse_program(mut self) -> Result<Ast, ParserError> {
        let mut modules = Vec::new();

        while !self.is_at_end() {
            let mut parsed = self.parse_module_group(None)?;
            modules.append(&mut parsed);
        }

        let callable_modules = canonicalize::collect_module_callable_signatures(&modules);
        for module in &mut modules {
            canonicalize::canonicalize_module_call_targets(module, &callable_modules);
        }

        Ok(Ast { modules })
    }

    pub(crate) fn expect(&mut self, kind: TokenKind, expected: &str) -> Result<(), ParserError> {
        self.expect_token(kind, expected).map(|_| ())
    }

    pub(crate) fn expect_token(
        &mut self,
        kind: TokenKind,
        expected: &str,
    ) -> Result<&'a Token, ParserError> {
        if self.check(kind) {
            Ok(self.advance().expect("expected token should be available"))
        } else {
            Err(self.expected(expected))
        }
    }

    pub(crate) fn expect_ident(&mut self, expected: &str) -> Result<String, ParserError> {
        if !self.check(TokenKind::Ident) {
            return Err(self.expected(expected));
        }

        Ok(self
            .advance()
            .expect("identifier token should be available")
            .lexeme()
            .to_string())
    }

    pub(crate) fn expected(&self, expected: &str) -> ParserError {
        let found = self
            .current()
            .map(|token| token.dump_label())
            .unwrap_or_else(|| "EOF".to_string());

        ParserError::at_current(
            format!("expected {expected}, found {found}"),
            self.current(),
        )
    }

    pub(crate) fn expect_block_end(
        &mut self,
        construct: &str,
        opening_span: Span,
    ) -> Result<(), ParserError> {
        if self.check(TokenKind::End) {
            self.advance();
            Ok(())
        } else if self.is_at_end() {
            Err(self.missing_end_error(construct, opening_span))
        } else {
            Err(self.expected("end"))
        }
    }

    pub(crate) fn expect_block_do(
        &mut self,
        construct: &str,
        opening_span: Span,
        hint: impl Into<String>,
    ) -> Result<(), ParserError> {
        if self.check(TokenKind::Do) {
            self.advance();
            Ok(())
        } else {
            Err(self.missing_do_error(construct, opening_span, hint))
        }
    }

    pub(crate) fn missing_end_error(&self, construct: &str, opening_span: Span) -> ParserError {
        ParserError::at_span(
            format!(
                "[E0003] unexpected end of file: missing 'end' to close {construct}. \
                 hint: add 'end' to finish {construct}"
            ),
            opening_span,
        )
    }

    pub(crate) fn missing_do_error(
        &self,
        construct: &str,
        opening_span: Span,
        hint: impl Into<String>,
    ) -> ParserError {
        let found = self
            .current()
            .map(|token| token.dump_label())
            .unwrap_or_else(|| "EOF".to_string());

        ParserError::at_span(
            format!(
                "[E0006] missing 'do' to start {construct}; found {found} instead. hint: {}",
                hint.into()
            ),
            opening_span,
        )
    }

    pub(crate) fn expect_clause_arrow(
        &mut self,
        clause: &str,
        clause_span: Span,
        hint: impl Into<String>,
    ) -> Result<(), ParserError> {
        if self.check(TokenKind::Arrow) {
            self.advance();
            Ok(())
        } else {
            Err(self.missing_arrow_error(clause, clause_span, hint))
        }
    }

    pub(crate) fn missing_arrow_error(
        &self,
        clause: &str,
        clause_span: Span,
        hint: impl Into<String>,
    ) -> ParserError {
        let found = self
            .current()
            .map(|token| token.dump_label())
            .unwrap_or_else(|| "EOF".to_string());
        let message = format!(
            "[E0007] missing '->' in {clause}; found {found} instead. hint: {}",
            hint.into()
        );

        if self.is_at_end() {
            ParserError::at_span(message, clause_span)
        } else {
            ParserError::at_current(message, self.current())
        }
    }

    pub(crate) fn missing_map_fat_arrow_error(
        &self,
        entry_kind: &str,
        hint: impl Into<String>,
    ) -> ParserError {
        let found = self
            .current()
            .map(|token| token.dump_label())
            .unwrap_or_else(|| "EOF".to_string());

        ParserError::at_current(
            format!(
                "[E0008] missing '=>' in {entry_kind}; found {found} instead. hint: {}",
                hint.into()
            ),
            self.current(),
        )
    }

    pub(crate) fn missing_comma_error(
        &self,
        list_kind: &str,
        hint: impl Into<String>,
    ) -> ParserError {
        let found = self
            .current()
            .map(|token| token.dump_label())
            .unwrap_or_else(|| "EOF".to_string());

        ParserError::at_current(
            format!(
                "[E0010] missing ',' in {list_kind}; found {found} instead. hint: {}",
                hint.into()
            ),
            self.current(),
        )
    }

    pub(crate) fn missing_comma_error_at_token(
        &self,
        list_kind: &str,
        token: &Token,
        hint: impl Into<String>,
    ) -> ParserError {
        ParserError::at_current(
            format!(
                "[E0010] missing ',' in {list_kind}; found {} instead. hint: {}",
                token.dump_label(),
                hint.into()
            ),
            Some(token),
        )
    }

    pub(crate) fn expect_closing_delimiter(
        &mut self,
        kind: TokenKind,
        expected: &str,
        construct: &str,
        opening_span: Span,
        hint: impl Into<String>,
    ) -> Result<(), ParserError> {
        if self.check(kind) {
            self.advance();
            Ok(())
        } else if self.current_ends_unclosed_delimiter_for(kind) {
            Err(self.unclosed_delimiter_error(construct, expected, opening_span, hint))
        } else {
            Err(self.expected(expected))
        }
    }

    pub(crate) fn expect_pattern_closing_delimiter(
        &mut self,
        kind: TokenKind,
        expected: &str,
        construct: &str,
        opening_span: Span,
        hint: impl Into<String>,
    ) -> Result<(), ParserError> {
        if self.check(kind) {
            self.advance();
            Ok(())
        } else if self.current_ends_pattern_unclosed_delimiter_for(kind) {
            Err(self.unclosed_delimiter_error(construct, expected, opening_span, hint))
        } else {
            Err(self.expected(expected))
        }
    }

    pub(crate) fn unclosed_delimiter_error(
        &self,
        construct: &str,
        expected: &str,
        opening_span: Span,
        hint: impl Into<String>,
    ) -> ParserError {
        ParserError::at_span(
            format!(
                "[E0002] unclosed delimiter: {construct} is missing '{expected}'. hint: {}",
                hint.into()
            ),
            opening_span,
        )
    }

    fn current_ends_unclosed_delimiter_for(&self, closing: TokenKind) -> bool {
        self.current_ends_unclosed_delimiter()
            || (closing == TokenKind::GtGt
                && self
                    .current()
                    .is_some_and(|token| token.kind() == TokenKind::Arrow))
    }

    fn current_ends_pattern_unclosed_delimiter_for(&self, closing: TokenKind) -> bool {
        self.current_ends_unclosed_delimiter_for(closing)
            || self
                .current()
                .is_some_and(|token| token.kind() == TokenKind::Arrow)
    }

    fn current_ends_unclosed_delimiter(&self) -> bool {
        self.current()
            .map(|token| {
                matches!(
                    token.kind(),
                    TokenKind::Do
                        | TokenKind::End
                        | TokenKind::Else
                        | TokenKind::Rescue
                        | TokenKind::Catch
                        | TokenKind::After
                        | TokenKind::Semicolon
                        | TokenKind::Eof
                )
            })
            .unwrap_or(true)
    }

    pub(crate) fn current_starts_missing_call_comma(&self) -> bool {
        self.current_starts_missing_expression_item_comma()
    }

    pub(crate) fn current_starts_missing_expression_item_comma(&self) -> bool {
        self.current().is_some_and(|token| {
            token_can_start_no_paren_arg(token.kind())
                || matches!(token.kind(), TokenKind::Minus | TokenKind::Not)
        })
    }

    pub(crate) fn current_starts_missing_map_entry_comma(&self) -> bool {
        self.current_starts_missing_keyword_entry_comma()
            || self.current_starts_missing_expression_item_comma()
    }

    pub(crate) fn current_starts_missing_param_comma(&self) -> bool {
        self.current_starts_missing_pattern_item_comma()
    }

    pub(crate) fn current_starts_missing_pattern_item_comma(&self) -> bool {
        self.current()
            .is_some_and(|token| token_can_start_pattern(token.kind()))
    }

    pub(crate) fn current_starts_missing_map_pattern_entry_comma(&self) -> bool {
        self.current_starts_missing_keyword_entry_comma()
            || self.current_starts_missing_pattern_item_comma()
    }

    pub(crate) fn current_starts_missing_bitstring_item_comma(&self) -> bool {
        self.current()
            .is_some_and(|token| token_can_start_no_paren_arg(token.kind()))
    }

    pub(crate) fn current_starts_missing_bitstring_pattern_comma(&self) -> bool {
        self.current()
            .is_some_and(|token| token_can_start_pattern(token.kind()))
    }

    pub(crate) fn current_starts_missing_with_clause_comma(&self) -> bool {
        self.current()
            .is_some_and(|token| token_can_start_pattern(token.kind()))
            && self.current_starts_clause_before_control_boundary(TokenKind::LeftArrow)
    }

    pub(crate) fn current_starts_missing_for_clause_comma(&self) -> bool {
        self.current().is_some_and(|token| {
            (token.kind() == TokenKind::Ident
                && self
                    .peek(1)
                    .is_some_and(|next| next.kind() == TokenKind::Colon))
                || (token_can_start_pattern(token.kind())
                    && self.current_starts_clause_before_control_boundary(TokenKind::LeftArrow))
        })
    }

    pub(crate) fn current_starts_missing_alias_child_comma(&self) -> bool {
        self.current_starts_module_reference()
    }

    pub(crate) fn current_starts_missing_keyword_entry_comma(&self) -> bool {
        self.starts_keyword_literal_entry()
    }

    fn current_starts_clause_before_control_boundary(&self, marker: TokenKind) -> bool {
        let mut paren_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut bitstring_depth = 0usize;

        for token in &self.tokens[self.index..] {
            match token.kind() {
                TokenKind::LParen => paren_depth += 1,
                TokenKind::RParen => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::LBrace => brace_depth += 1,
                TokenKind::RBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenKind::LBracket => bracket_depth += 1,
                TokenKind::RBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::LtLt => bitstring_depth += 1,
                TokenKind::GtGt => bitstring_depth = bitstring_depth.saturating_sub(1),
                kind if paren_depth == 0
                    && brace_depth == 0
                    && bracket_depth == 0
                    && bitstring_depth == 0
                    && kind == marker =>
                {
                    return true;
                }
                TokenKind::Comma
                | TokenKind::Do
                | TokenKind::Else
                | TokenKind::End
                | TokenKind::Semicolon
                | TokenKind::Eof
                    if paren_depth == 0
                        && brace_depth == 0
                        && bracket_depth == 0
                        && bitstring_depth == 0 =>
                {
                    return false;
                }
                _ => {}
            }
        }

        false
    }

    fn anonymous_fn_clause_signature_example(arity: usize) -> String {
        match arity {
            0 => "-> ...".to_string(),
            1 => "value -> ...".to_string(),
            2 => "left, right -> ...".to_string(),
            _ => format!(
                "{} -> ...",
                (1..=arity)
                    .map(|index| format!("arg{index}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    pub(crate) fn missing_named_capture_arity_error(
        &self,
        offset: usize,
        target: &str,
    ) -> ParserError {
        ParserError::at_span(
            format!(
                "[E0009] missing '/arity' in named function capture `&{target}`. hint: write `&{target}/arity`, for example `&{target}/2` if the function takes two arguments"
            ),
            Span::new(offset, offset + 1),
        )
    }

    pub(crate) fn empty_capture_expression_error(&self, offset: usize) -> ParserError {
        ParserError::at_span(
            "[E0009] empty capture expression `&()`. hint: wrap an expression that uses placeholders, for example `&(&1 + 1)` or `&(expr_with_&1)`",
            Span::new(offset, offset + 1),
        )
    }

    pub(crate) fn invalid_capture_placeholder_error(
        &self,
        offset: usize,
        placeholder: usize,
    ) -> ParserError {
        ParserError::at_span(
            format!(
                "[E0009] invalid capture placeholder `&{placeholder}`. hint: capture placeholders start at `&1`; replace `&{placeholder}` with `&1` or another positive index"
            ),
            Span::new(offset, offset + 1),
        )
    }

    pub(crate) fn anonymous_function_clause_arity_mismatch_error(
        &self,
        clause_span: Span,
        expected_arity: usize,
        found_arity: usize,
    ) -> ParserError {
        let expected_label = if expected_arity == 1 {
            "parameter"
        } else {
            "parameters"
        };
        let found_label = if found_arity == 1 {
            "parameter"
        } else {
            "parameters"
        };
        let example = Self::anonymous_fn_clause_signature_example(expected_arity);

        ParserError::at_span(
            format!(
                "[E0009] anonymous function clause arity mismatch: the first clause takes {expected_arity} {expected_label}, but this clause takes {found_arity} {found_label}. hint: make every clause in the same 'fn' use the same arity, for example `{example}`"
            ),
            clause_span,
        )
    }

    pub(crate) fn unexpected_arrow_error(&self) -> ParserError {
        ParserError::at_current(
            "[E0004] unexpected '->' outside a valid branch. hint: use 'fn ... -> ... end' for anonymous functions, or move '->' into a branch inside case/cond/with/for/try",
            self.current(),
        )
    }

    pub(crate) fn unexpected_block_keyword_error(&self) -> Option<ParserError> {
        let token = self.current()?;
        let message = match token.kind() {
            TokenKind::Else => {
                "[E0005] unexpected 'else' without a matching block. hint: move 'else' inside an 'if', 'unless', or 'with' expression, or remove the extra 'else'"
            }
            TokenKind::Rescue => {
                "[E0005] unexpected 'rescue' without a matching 'try'. hint: move 'rescue' inside a 'try ... end' expression, add the missing 'try', or remove the extra 'rescue'"
            }
            TokenKind::Catch => {
                "[E0005] unexpected 'catch' without a matching 'try'. hint: move 'catch' inside a 'try ... end' expression, add the missing 'try', or remove the extra 'catch'"
            }
            TokenKind::After => {
                "[E0005] unexpected 'after' without a matching 'try'. hint: move 'after' inside a 'try ... end' expression, add the missing 'try', or remove the extra 'after'"
            }
            TokenKind::End => {
                "[E0005] unexpected 'end' without an opening block. hint: remove the extra 'end', or add the missing block opener before this point"
            }
            TokenKind::Do => {
                "[E0005] unexpected 'do' without a block header. hint: put 'do' after a block opener like 'def', 'if', 'case', 'cond', 'with', 'for', or 'try', or remove the extra 'do'"
            }
            _ => return None,
        };

        Some(ParserError::at_current(message, Some(token)))
    }

    pub(crate) fn check(&self, kind: TokenKind) -> bool {
        self.current()
            .map(|token| token.kind() == kind)
            .unwrap_or(false)
    }

    pub(crate) fn match_kind(&mut self, kind: TokenKind) -> bool {
        if !self.check(kind) {
            return false;
        }

        self.index += 1;
        true
    }

    pub(crate) fn advance(&mut self) -> Option<&'a Token> {
        let token = self.current()?;
        self.index += 1;
        Some(token)
    }

    pub(crate) fn current(&self) -> Option<&'a Token> {
        self.tokens.get(self.index)
    }

    pub(crate) fn peek(&self, distance: usize) -> Option<&'a Token> {
        self.tokens.get(self.index + distance)
    }

    pub(crate) fn is_at_end(&self) -> bool {
        self.check(TokenKind::Eof)
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_forms;
#[cfg(test)]
mod tests_modules;
