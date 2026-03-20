use crate::lexer::{Token, TokenKind};

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
        "ok" | "err" | "tuple" | "list" | "map" | "keyword" | "protocol_dispatch" | "host_call"
            | "abs" | "length" | "hd" | "tl" | "elem" | "tuple_size" | "to_string"
            | "max" | "min" | "round" | "trunc"
            | "map_size" | "put_elem" | "inspect"
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
