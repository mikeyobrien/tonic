use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn start(self) -> usize {
        self.start
    }

    pub fn end(self) -> usize {
        self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    kind: TokenKind,
    lexeme: String,
    span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Defmodule,
    Def,
    Defp,
    Do,
    End,
    If,
    Unless,
    Case,
    Cond,
    With,
    For,
    Fn,
    Else,
    Try,
    Rescue,
    Catch,
    After,
    Raise,
    True,
    False,
    Nil,
    And,
    Or,
    Not,
    In,
    When,
    Ident,
    Atom,
    Integer,
    Float,
    String,
    StringStart,
    StringPart,
    InterpolationStart,
    InterpolationEnd,
    StringEnd,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Percent,
    At,
    Colon,
    Comma,
    Semicolon,
    Dot,
    DotDot,
    Caret,
    Plus,
    PlusPlus,
    Minus,
    MinusMinus,
    Star,
    Slash,
    MatchEq,
    EqEq,
    BangEq,
    Bang,
    Lt,
    LtEq,
    Gt,
    GtEq,
    LessGreater,
    Question,
    Pipe,
    PipeGt,
    FatArrow,
    Arrow,
    LeftArrow,
    BackslashBackslash,
    Ampersand,
    AndAnd,
    AmpAmpAmp,
    OrOr,
    PipePipePipe,
    CaretCaretCaret,
    TildeTildeTilde,
    LtLt,
    GtGt,
    LtLtLt,
    GtGtGt,
    StrictEq,
    StrictBangEq,
    SlashSlash,
    Eof,
}

impl Token {
    fn simple(kind: TokenKind, span: Span) -> Self {
        Self {
            kind,
            lexeme: String::new(),
            span,
        }
    }

    fn with_lexeme(kind: TokenKind, lexeme: impl Into<String>, span: Span) -> Self {
        Self {
            kind,
            lexeme: lexeme.into(),
            span,
        }
    }

    pub fn kind(&self) -> TokenKind {
        self.kind
    }

    pub fn lexeme(&self) -> &str {
        &self.lexeme
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn dump_record(&self) -> DumpToken<'_> {
        DumpToken {
            kind: self.kind.dump_name(),
            lexeme: self.lexeme(),
            span_start: self.span.start(),
            span_end: self.span.end(),
        }
    }

    pub fn dump_label(&self) -> String {
        match self.kind {
            TokenKind::Defmodule
            | TokenKind::Def
            | TokenKind::Defp
            | TokenKind::Do
            | TokenKind::End
            | TokenKind::If
            | TokenKind::Unless
            | TokenKind::Case
            | TokenKind::Cond
            | TokenKind::With
            | TokenKind::For
            | TokenKind::Fn
            | TokenKind::Else
            | TokenKind::Try
            | TokenKind::Rescue
            | TokenKind::Catch
            | TokenKind::After
            | TokenKind::Raise
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Nil
            | TokenKind::And
            | TokenKind::Or
            | TokenKind::Not
            | TokenKind::In
            | TokenKind::When
            | TokenKind::Ident
            | TokenKind::Atom
            | TokenKind::Integer
            | TokenKind::Float
            | TokenKind::String
            | TokenKind::StringPart => {
                format!("{}({})", self.kind.dump_name(), self.lexeme)
            }
            _ => self.kind.dump_name().to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DumpToken<'a> {
    pub kind: &'a str,
    pub lexeme: &'a str,
    pub span_start: usize,
    pub span_end: usize,
}

impl TokenKind {
    pub fn dump_name(self) -> &'static str {
        match self {
            TokenKind::Defmodule => "DEFMODULE",
            TokenKind::Def => "DEF",
            TokenKind::Defp => "DEFP",
            TokenKind::Do => "DO",
            TokenKind::End => "END",
            TokenKind::If => "IF",
            TokenKind::Unless => "UNLESS",
            TokenKind::Case => "CASE",
            TokenKind::Cond => "COND",
            TokenKind::With => "WITH",
            TokenKind::For => "FOR",
            TokenKind::Fn => "FN",
            TokenKind::Else => "ELSE",
            TokenKind::Try => "TRY",
            TokenKind::Rescue => "RESCUE",
            TokenKind::Catch => "CATCH",
            TokenKind::After => "AFTER",
            TokenKind::Raise => "RAISE",
            TokenKind::True => "TRUE",
            TokenKind::False => "FALSE",
            TokenKind::Nil => "NIL",
            TokenKind::And => "AND",
            TokenKind::Or => "OR",
            TokenKind::Not => "NOT",
            TokenKind::In => "IN",
            TokenKind::When => "WHEN",
            TokenKind::Ident => "IDENT",
            TokenKind::Atom => "ATOM",
            TokenKind::Integer => "INT",
            TokenKind::Float => "FLOAT",
            TokenKind::String => "STRING",
            TokenKind::StringStart => "STRING_START",
            TokenKind::StringPart => "STRING_PART",
            TokenKind::InterpolationStart => "INTERPOLATION_START",
            TokenKind::InterpolationEnd => "INTERPOLATION_END",
            TokenKind::StringEnd => "STRING_END",
            TokenKind::LParen => "LPAREN",
            TokenKind::RParen => "RPAREN",
            TokenKind::LBrace => "LBRACE",
            TokenKind::RBrace => "RBRACE",
            TokenKind::LBracket => "LBRACKET",
            TokenKind::RBracket => "RBRACKET",
            TokenKind::Percent => "PERCENT",
            TokenKind::At => "AT",
            TokenKind::Colon => "COLON",
            TokenKind::Comma => "COMMA",
            TokenKind::Semicolon => "SEMICOLON",
            TokenKind::Dot => "DOT",
            TokenKind::DotDot => "DOT_DOT",
            TokenKind::Caret => "CARET",
            TokenKind::Plus => "PLUS",
            TokenKind::PlusPlus => "PLUS_PLUS",
            TokenKind::Minus => "MINUS",
            TokenKind::MinusMinus => "MINUS_MINUS",
            TokenKind::Star => "STAR",
            TokenKind::Slash => "SLASH",
            TokenKind::MatchEq => "MATCH_EQ",
            TokenKind::EqEq => "EQ_EQ",
            TokenKind::BangEq => "BANG_EQ",
            TokenKind::Bang => "BANG",
            TokenKind::Lt => "LT",
            TokenKind::LtEq => "LT_EQ",
            TokenKind::Gt => "GT",
            TokenKind::GtEq => "GT_EQ",
            TokenKind::LessGreater => "LESS_GREATER",
            TokenKind::Question => "QUESTION",
            TokenKind::Pipe => "PIPE",
            TokenKind::PipeGt => "PIPE_GT",
            TokenKind::FatArrow => "FAT_ARROW",
            TokenKind::Arrow => "ARROW",
            TokenKind::LeftArrow => "LEFT_ARROW",
            TokenKind::BackslashBackslash => "BACKSLASH_BACKSLASH",
            TokenKind::Ampersand => "AMPERSAND",
            TokenKind::AndAnd => "AND_AND",
            TokenKind::AmpAmpAmp => "AMP_AMP_AMP",
            TokenKind::OrOr => "OR_OR",
            TokenKind::PipePipePipe => "PIPE_PIPE_PIPE",
            TokenKind::CaretCaretCaret => "CARET_CARET_CARET",
            TokenKind::TildeTildeTilde => "TILDE_TILDE_TILDE",
            TokenKind::LtLt => "LT_LT",
            TokenKind::GtGt => "GT_GT",
            TokenKind::LtLtLt => "LT_LT_LT",
            TokenKind::GtGtGt => "GT_GT_GT",
            TokenKind::StrictEq => "STRICT_EQ",
            TokenKind::StrictBangEq => "STRICT_BANG_EQ",
            TokenKind::SlashSlash => "SLASH_SLASH",
            TokenKind::Eof => "EOF",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexerError {
    kind: LexerErrorKind,
    span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LexerErrorKind {
    InvalidToken(char),
    UnterminatedString,
    /// No digits follow a radix prefix (e.g. `0x`, `0o`, `0b` with nothing after)
    EmptyNumericLiteral {
        prefix: &'static str,
    },
    /// A digit is invalid for the given base (e.g. `0b12` — `2` is not binary)
    InvalidDigitForBase {
        digit: char,
        base: &'static str,
    },
    /// Underscore separator at start or end of digit sequence (e.g. `0x_FF` or `0xFF_`)
    MisplacedNumericSeparator,
}

impl LexerError {
    fn invalid_token(value: char, span: Span) -> Self {
        Self {
            kind: LexerErrorKind::InvalidToken(value),
            span,
        }
    }

    fn unterminated_string(span: Span) -> Self {
        Self {
            kind: LexerErrorKind::UnterminatedString,
            span,
        }
    }

    fn empty_numeric_literal(prefix: &'static str, span: Span) -> Self {
        Self {
            kind: LexerErrorKind::EmptyNumericLiteral { prefix },
            span,
        }
    }

    fn invalid_digit_for_base(digit: char, base: &'static str, span: Span) -> Self {
        Self {
            kind: LexerErrorKind::InvalidDigitForBase { digit, base },
            span,
        }
    }

    fn misplaced_numeric_separator(span: Span) -> Self {
        Self {
            kind: LexerErrorKind::MisplacedNumericSeparator,
            span,
        }
    }

    pub fn offset(&self) -> usize {
        self.span.start()
    }

    #[cfg(test)]
    pub fn span(&self) -> Span {
        self.span
    }
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            LexerErrorKind::InvalidToken(value) => {
                write!(f, "invalid token '{value}' at offset {}", self.span.start)
            }
            LexerErrorKind::UnterminatedString => {
                write!(
                    f,
                    "unterminated string literal at offset {}",
                    self.span.start
                )
            }
            LexerErrorKind::EmptyNumericLiteral { prefix } => {
                write!(
                    f,
                    "numeric literal '{prefix}' has no digits at offset {}",
                    self.span.start
                )
            }
            LexerErrorKind::InvalidDigitForBase { digit, base } => {
                write!(
                    f,
                    "invalid {base} digit '{digit}' at offset {}",
                    self.span.start
                )
            }
            LexerErrorKind::MisplacedNumericSeparator => {
                write!(
                    f,
                    "numeric separator '_' cannot appear at start or end of digits at offset {}",
                    self.span.start
                )
            }
        }
    }
}

impl std::error::Error for LexerError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexerState {
    Normal,
    String {
        is_heredoc: bool,
        brace_depth: usize,
    },
}

pub fn scan_tokens(source: &str) -> Result<Vec<Token>, LexerError> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut idx = 0;

    let mut state_stack = vec![LexerState::Normal];
    let mut current_brace_depth: usize = 0;

    while idx < chars.len() {
        let current = chars[idx];

        match state_stack.last().unwrap() {
            LexerState::Normal => {
                if current.is_whitespace() {
                    idx += 1;
                    continue;
                }

                if current == '#' {
                    idx += 1;
                    while idx < chars.len() && chars[idx] != '\n' {
                        idx += 1;
                    }
                    continue;
                }

                match current {
                    '(' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::LParen, Span::new(start, idx)));
                    }
                    ')' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::RParen, Span::new(start, idx)));
                    }
                    '{' => {
                        let start = idx;
                        idx += 1;
                        current_brace_depth += 1;
                        tokens.push(Token::simple(TokenKind::LBrace, Span::new(start, idx)));
                    }
                    '}' => {
                        let start = idx;
                        idx += 1;
                        current_brace_depth = current_brace_depth.saturating_sub(1);

                        if state_stack.len() > 1 {
                            if let LexerState::String { brace_depth, .. } =
                                state_stack[state_stack.len() - 2]
                            {
                                if current_brace_depth == brace_depth {
                                    state_stack.pop();
                                    tokens.push(Token::simple(
                                        TokenKind::InterpolationEnd,
                                        Span::new(start, idx),
                                    ));
                                    continue;
                                }
                            }
                        }
                        tokens.push(Token::simple(TokenKind::RBrace, Span::new(start, idx)));
                    }
                    '[' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::LBracket, Span::new(start, idx)));
                    }
                    ']' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::RBracket, Span::new(start, idx)));
                    }
                    '%' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::Percent, Span::new(start, idx)));
                    }
                    '@' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::At, Span::new(start, idx)));
                    }
                    ',' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::Comma, Span::new(start, idx)));
                    }
                    ';' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::Semicolon, Span::new(start, idx)));
                    }
                    '.' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'.') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::DotDot, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Dot, Span::new(start, idx)));
                        }
                    }
                    '^' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'^') && chars.get(idx + 2) == Some(&'^') {
                            idx += 3;
                            tokens.push(Token::simple(
                                TokenKind::CaretCaretCaret,
                                Span::new(start, idx),
                            ));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Caret, Span::new(start, idx)));
                        }
                    }
                    '+' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'+') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::PlusPlus, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Plus, Span::new(start, idx)));
                        }
                    }
                    '-' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'>') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::Arrow, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'-') {
                            idx += 2;
                            tokens
                                .push(Token::simple(TokenKind::MinusMinus, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Minus, Span::new(start, idx)));
                        }
                    }
                    '*' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::Star, Span::new(start, idx)));
                    }
                    '/' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'/') {
                            idx += 2;
                            tokens
                                .push(Token::simple(TokenKind::SlashSlash, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Slash, Span::new(start, idx)));
                        }
                    }
                    '\\' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'\\') {
                            idx += 2;
                            tokens.push(Token::simple(
                                TokenKind::BackslashBackslash,
                                Span::new(start, idx),
                            ));
                        } else {
                            return Err(LexerError::invalid_token(
                                '\\',
                                Span::new(start, start + 1),
                            ));
                        }
                    }
                    '=' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'>') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::FatArrow, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'=')
                            && chars.get(idx + 2) == Some(&'=')
                        {
                            idx += 3;
                            tokens.push(Token::simple(TokenKind::StrictEq, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'=') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::EqEq, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::MatchEq, Span::new(start, idx)));
                        }
                    }
                    '!' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'=') && chars.get(idx + 2) == Some(&'=') {
                            idx += 3;
                            tokens.push(Token::simple(
                                TokenKind::StrictBangEq,
                                Span::new(start, idx),
                            ));
                        } else if chars.get(idx + 1) == Some(&'=') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::BangEq, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Bang, Span::new(start, idx)));
                        }
                    }
                    '<' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'<') && chars.get(idx + 2) == Some(&'<') {
                            idx += 3;
                            tokens.push(Token::simple(TokenKind::LtLtLt, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'<') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::LtLt, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'=') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::LtEq, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'-') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::LeftArrow, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'>') {
                            idx += 2;
                            tokens
                                .push(Token::simple(TokenKind::LessGreater, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Lt, Span::new(start, idx)));
                        }
                    }
                    '>' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'>') && chars.get(idx + 2) == Some(&'>') {
                            idx += 3;
                            tokens.push(Token::simple(TokenKind::GtGtGt, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'>') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::GtGt, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'=') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::GtEq, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Gt, Span::new(start, idx)));
                        }
                    }
                    '?' => {
                        let start = idx;
                        idx += 1;
                        // Char literal: ?a, ?\n, etc.
                        if idx < chars.len()
                            && chars[idx] != ' '
                            && chars[idx] != '\n'
                            && chars[idx] != '\t'
                            && chars[idx] != ')'
                            && chars[idx] != ','
                            && chars[idx] != ']'
                            && chars[idx] != '}'
                        {
                            // Check if this looks like a char literal rather than the ? operator.
                            // The ? operator is always followed by whitespace, closing delimiters, or EOF in expression position.
                            // Char literals are ?<char> or ?\<escape>.
                            let char_value: u32;
                            if chars[idx] == '\\' && idx + 1 < chars.len() {
                                // Escape sequence
                                idx += 1;
                                char_value = match chars[idx] {
                                    'n' => 10,
                                    't' => 9,
                                    'r' => 13,
                                    's' => 32,
                                    '\\' => 92,
                                    '"' => 34,
                                    '\'' => 39,
                                    '0' => 0,
                                    other => other as u32,
                                };
                                idx += 1;
                            } else {
                                char_value = chars[idx] as u32;
                                idx += 1;
                            }
                            let lexeme = char_value.to_string();
                            tokens.push(Token::with_lexeme(
                                TokenKind::Integer,
                                lexeme,
                                Span::new(start, idx),
                            ));
                        } else {
                            tokens.push(Token::simple(TokenKind::Question, Span::new(start, idx)));
                        }
                    }
                    '|' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'>') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::PipeGt, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'|')
                            && chars.get(idx + 2) == Some(&'|')
                        {
                            idx += 3;
                            tokens.push(Token::simple(
                                TokenKind::PipePipePipe,
                                Span::new(start, idx),
                            ));
                        } else if chars.get(idx + 1) == Some(&'|') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::OrOr, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Pipe, Span::new(start, idx)));
                        }
                    }
                    '&' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'&') && chars.get(idx + 2) == Some(&'&') {
                            idx += 3;
                            tokens.push(Token::simple(TokenKind::AmpAmpAmp, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'&') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::AndAnd, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Ampersand, Span::new(start, idx)));
                        }
                    }
                    ':' => {
                        let start = idx;

                        if chars.get(idx + 1).is_some_and(|next| is_ident_start(*next)) {
                            idx += 1;
                            let atom_start = idx;
                            idx += 1;

                            while idx < chars.len() && is_ident_continue(chars[idx]) {
                                idx += 1;
                            }

                            let lexeme: String = chars[atom_start..idx].iter().collect();
                            tokens.push(Token::with_lexeme(
                                TokenKind::Atom,
                                lexeme,
                                Span::new(start, idx),
                            ));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Colon, Span::new(start, idx)));
                        }
                    }
                    '~' => {
                        let start = idx;
                        // ~~~ is bitwise not (unary operator)
                        if chars.get(idx + 1) == Some(&'~') && chars.get(idx + 2) == Some(&'~') {
                            idx += 3;
                            tokens.push(Token::simple(
                                TokenKind::TildeTildeTilde,
                                Span::new(start, idx),
                            ));
                            continue;
                        }
                        let Some(sigil_kind) = chars.get(idx + 1).copied() else {
                            return Err(LexerError::invalid_token(
                                '~',
                                Span::new(start, start + 1),
                            ));
                        };

                        if !matches!(sigil_kind, 's' | 'r' | 'w') {
                            return Err(LexerError::invalid_token(
                                '~',
                                Span::new(start, start + 1),
                            ));
                        }

                        let Some(open_delim) = chars.get(idx + 2).copied() else {
                            return Err(LexerError::invalid_token(
                                '~',
                                Span::new(start, start + 1),
                            ));
                        };

                        let close_delim = match open_delim {
                            '(' => ')',
                            '[' => ']',
                            '{' => '}',
                            '<' => '>',
                            other => other,
                        };

                        idx += 3;
                        let content_start = idx;
                        while idx < chars.len() && chars[idx] != close_delim {
                            idx += 1;
                        }

                        if idx >= chars.len() {
                            return Err(LexerError::unterminated_string(Span::new(
                                start,
                                chars.len(),
                            )));
                        }

                        let lexeme: String = chars[content_start..idx].iter().collect();
                        idx += 1;

                        if sigil_kind == 'w' {
                            // Check for optional modifier after closing delimiter (e.g. `a` for atoms)
                            let use_atoms = chars.get(idx) == Some(&'a');
                            if use_atoms {
                                idx += 1;
                            }

                            // Split content on whitespace and emit a list literal token sequence
                            let words: Vec<&str> = lexeme.split_whitespace().collect();
                            tokens.push(Token::simple(TokenKind::LBracket, Span::new(start, idx)));
                            for (i, word) in words.iter().enumerate() {
                                if i > 0 {
                                    tokens.push(Token::simple(
                                        TokenKind::Comma,
                                        Span::new(start, idx),
                                    ));
                                }
                                if use_atoms {
                                    tokens.push(Token::with_lexeme(
                                        TokenKind::Atom,
                                        word.to_string(),
                                        Span::new(start, idx),
                                    ));
                                } else {
                                    tokens.push(Token::with_lexeme(
                                        TokenKind::String,
                                        word.to_string(),
                                        Span::new(start, idx),
                                    ));
                                }
                            }
                            tokens.push(Token::simple(TokenKind::RBracket, Span::new(start, idx)));
                        } else {
                            tokens.push(Token::with_lexeme(
                                TokenKind::String,
                                lexeme,
                                Span::new(start, idx),
                            ));
                        }
                    }
                    '"' => {
                        let start = idx;
                        let is_heredoc =
                            chars.get(idx + 1) == Some(&'"') && chars.get(idx + 2) == Some(&'"');

                        let mut has_interpolation = false;
                        let mut temp_idx = if is_heredoc { idx + 3 } else { idx + 1 };

                        if is_heredoc {
                            while temp_idx < chars.len() {
                                if chars.get(temp_idx) == Some(&'"')
                                    && chars.get(temp_idx + 1) == Some(&'"')
                                    && chars.get(temp_idx + 2) == Some(&'"')
                                {
                                    break;
                                }
                                if chars.get(temp_idx) == Some(&'#')
                                    && chars.get(temp_idx + 1) == Some(&'{')
                                {
                                    has_interpolation = true;
                                    break;
                                }
                                temp_idx += 1;
                            }
                        } else {
                            while temp_idx < chars.len() {
                                let peek = chars[temp_idx];
                                if peek == '"' {
                                    break;
                                }
                                if peek == '#' && chars.get(temp_idx + 1) == Some(&'{') {
                                    has_interpolation = true;
                                    break;
                                }
                                temp_idx += 1;
                            }
                        }

                        if has_interpolation {
                            let end_idx = if is_heredoc { idx + 3 } else { idx + 1 };
                            tokens.push(Token::simple(
                                TokenKind::StringStart,
                                Span::new(start, end_idx),
                            ));
                            state_stack.push(LexerState::String {
                                is_heredoc,
                                brace_depth: current_brace_depth,
                            });
                            idx = end_idx;
                        } else {
                            let mut literal = String::new();
                            let mut terminated = false;

                            if is_heredoc {
                                idx += 3;

                                while idx < chars.len() {
                                    if chars.get(idx) == Some(&'"')
                                        && chars.get(idx + 1) == Some(&'"')
                                        && chars.get(idx + 2) == Some(&'"')
                                    {
                                        terminated = true;
                                        idx += 3;
                                        break;
                                    }

                                    literal.push(chars[idx]);
                                    idx += 1;
                                }
                            } else {
                                idx += 1;

                                while idx < chars.len() {
                                    let peek = chars[idx];

                                    if peek == '"' {
                                        terminated = true;
                                        idx += 1;
                                        break;
                                    }

                                    literal.push(peek);
                                    idx += 1;
                                }
                            }

                            if !terminated {
                                return Err(LexerError::unterminated_string(Span::new(
                                    start,
                                    chars.len(),
                                )));
                            }

                            tokens.push(Token::with_lexeme(
                                TokenKind::String,
                                literal,
                                Span::new(start, idx),
                            ));
                        }
                    }
                    value if value.is_ascii_digit() => {
                        let start = idx;
                        idx += 1;

                        // Check for radix prefix: 0x, 0o, 0b
                        if value == '0' && idx < chars.len() {
                            match chars[idx] {
                                'x' | 'X' => {
                                    idx += 1; // skip 'x'/'X'
                                              // Error: no digits follow the prefix
                                    if idx >= chars.len()
                                        || (!chars[idx].is_ascii_hexdigit() && chars[idx] != '_')
                                    {
                                        return Err(LexerError::empty_numeric_literal(
                                            "0x",
                                            Span::new(start, idx),
                                        ));
                                    }
                                    // Error: separator at start
                                    if chars[idx] == '_' {
                                        return Err(LexerError::misplaced_numeric_separator(
                                            Span::new(start, idx + 1),
                                        ));
                                    }
                                    let digit_start = idx;
                                    while idx < chars.len()
                                        && (chars[idx].is_ascii_hexdigit() || chars[idx] == '_')
                                    {
                                        idx += 1;
                                    }
                                    // Error: separator at end
                                    if chars[idx - 1] == '_' {
                                        return Err(LexerError::misplaced_numeric_separator(
                                            Span::new(start, idx),
                                        ));
                                    }
                                    let digits: String = chars[digit_start..idx]
                                        .iter()
                                        .filter(|c| **c != '_')
                                        .collect();
                                    let int_value =
                                        i64::from_str_radix(&digits, 16).map_err(|_| {
                                            LexerError::empty_numeric_literal(
                                                "0x",
                                                Span::new(start, idx),
                                            )
                                        })?;
                                    tokens.push(Token::with_lexeme(
                                        TokenKind::Integer,
                                        int_value.to_string(),
                                        Span::new(start, idx),
                                    ));
                                    continue;
                                }
                                'o' | 'O' => {
                                    idx += 1; // skip 'o'/'O'
                                              // Error: no digits follow the prefix
                                    if idx >= chars.len()
                                        || (!('0'..='7').contains(&chars[idx]) && chars[idx] != '_')
                                    {
                                        // Check for invalid digit (e.g. 0o8)
                                        if idx < chars.len() && chars[idx].is_ascii_digit() {
                                            return Err(LexerError::invalid_digit_for_base(
                                                chars[idx],
                                                "octal",
                                                Span::new(start, idx + 1),
                                            ));
                                        }
                                        return Err(LexerError::empty_numeric_literal(
                                            "0o",
                                            Span::new(start, idx),
                                        ));
                                    }
                                    // Error: separator at start
                                    if chars[idx] == '_' {
                                        return Err(LexerError::misplaced_numeric_separator(
                                            Span::new(start, idx + 1),
                                        ));
                                    }
                                    let digit_start = idx;
                                    while idx < chars.len() {
                                        if chars[idx] == '_'
                                            || (chars[idx].is_ascii_digit() && chars[idx] <= '7')
                                        {
                                            idx += 1;
                                        } else if chars[idx].is_ascii_digit() {
                                            // digit 8 or 9 in octal literal
                                            return Err(LexerError::invalid_digit_for_base(
                                                chars[idx],
                                                "octal",
                                                Span::new(start, idx + 1),
                                            ));
                                        } else {
                                            break;
                                        }
                                    }
                                    // Error: separator at end
                                    if chars[idx - 1] == '_' {
                                        return Err(LexerError::misplaced_numeric_separator(
                                            Span::new(start, idx),
                                        ));
                                    }
                                    let digits: String = chars[digit_start..idx]
                                        .iter()
                                        .filter(|c| **c != '_')
                                        .collect();
                                    let int_value =
                                        i64::from_str_radix(&digits, 8).map_err(|_| {
                                            LexerError::empty_numeric_literal(
                                                "0o",
                                                Span::new(start, idx),
                                            )
                                        })?;
                                    tokens.push(Token::with_lexeme(
                                        TokenKind::Integer,
                                        int_value.to_string(),
                                        Span::new(start, idx),
                                    ));
                                    continue;
                                }
                                'b' | 'B' => {
                                    idx += 1; // skip 'b'/'B'
                                              // Error: no digits follow the prefix
                                    if idx >= chars.len()
                                        || (chars[idx] != '0'
                                            && chars[idx] != '1'
                                            && chars[idx] != '_')
                                    {
                                        // Check for invalid digit (e.g. 0b2)
                                        if idx < chars.len() && chars[idx].is_ascii_digit() {
                                            return Err(LexerError::invalid_digit_for_base(
                                                chars[idx],
                                                "binary",
                                                Span::new(start, idx + 1),
                                            ));
                                        }
                                        return Err(LexerError::empty_numeric_literal(
                                            "0b",
                                            Span::new(start, idx),
                                        ));
                                    }
                                    // Error: separator at start
                                    if chars[idx] == '_' {
                                        return Err(LexerError::misplaced_numeric_separator(
                                            Span::new(start, idx + 1),
                                        ));
                                    }
                                    let digit_start = idx;
                                    while idx < chars.len() {
                                        if chars[idx] == '_' || matches!(chars[idx], '0' | '1') {
                                            idx += 1;
                                        } else if chars[idx].is_ascii_digit() {
                                            // digit 2-9 in binary literal
                                            return Err(LexerError::invalid_digit_for_base(
                                                chars[idx],
                                                "binary",
                                                Span::new(start, idx + 1),
                                            ));
                                        } else {
                                            break;
                                        }
                                    }
                                    // Error: separator at end
                                    if chars[idx - 1] == '_' {
                                        return Err(LexerError::misplaced_numeric_separator(
                                            Span::new(start, idx),
                                        ));
                                    }
                                    let digits: String = chars[digit_start..idx]
                                        .iter()
                                        .filter(|c| **c != '_')
                                        .collect();
                                    let int_value =
                                        i64::from_str_radix(&digits, 2).map_err(|_| {
                                            LexerError::empty_numeric_literal(
                                                "0b",
                                                Span::new(start, idx),
                                            )
                                        })?;
                                    tokens.push(Token::with_lexeme(
                                        TokenKind::Integer,
                                        int_value.to_string(),
                                        Span::new(start, idx),
                                    ));
                                    continue;
                                }
                                _ => {}
                            }
                        }

                        // Decimal integer: consume digits and underscores
                        while idx < chars.len()
                            && (chars[idx].is_ascii_digit() || chars[idx] == '_')
                        {
                            idx += 1;
                        }

                        // Error: separator at end of integer part
                        if chars[idx - 1] == '_' {
                            return Err(LexerError::misplaced_numeric_separator(Span::new(
                                start, idx,
                            )));
                        }

                        let mut kind = TokenKind::Integer;
                        if idx + 1 < chars.len()
                            && chars[idx] == '.'
                            && chars[idx + 1].is_ascii_digit()
                        {
                            kind = TokenKind::Float;
                            idx += 1;

                            while idx < chars.len()
                                && (chars[idx].is_ascii_digit() || chars[idx] == '_')
                            {
                                idx += 1;
                            }

                            // Error: separator at end of fractional part
                            if chars[idx - 1] == '_' {
                                return Err(LexerError::misplaced_numeric_separator(Span::new(
                                    start, idx,
                                )));
                            }
                        }

                        // Strip underscores from the lexeme
                        let lexeme: String =
                            chars[start..idx].iter().filter(|c| **c != '_').collect();
                        tokens.push(Token::with_lexeme(kind, lexeme, Span::new(start, idx)));
                    }
                    value if is_ident_start(value) => {
                        let start = idx;
                        idx += 1;

                        while idx < chars.len() && is_ident_continue(chars[idx]) {
                            idx += 1;
                        }

                        let lexeme: String = chars[start..idx].iter().collect();
                        let kind = keyword_kind(&lexeme).unwrap_or(TokenKind::Ident);
                        tokens.push(Token::with_lexeme(kind, lexeme, Span::new(start, idx)));
                    }
                    unexpected => {
                        return Err(LexerError::invalid_token(
                            unexpected,
                            Span::new(idx, idx + 1),
                        ));
                    }
                }
            }
            LexerState::String {
                is_heredoc,
                brace_depth: _,
            } => {
                let start = idx;
                let mut literal = String::new();
                let mut terminated = false;
                let mut is_interpolation = false;

                if *is_heredoc {
                    while idx < chars.len() {
                        if chars.get(idx) == Some(&'"')
                            && chars.get(idx + 1) == Some(&'"')
                            && chars.get(idx + 2) == Some(&'"')
                        {
                            terminated = true;
                            idx += 3;
                            break;
                        }
                        if chars.get(idx) == Some(&'#') && chars.get(idx + 1) == Some(&'{') {
                            is_interpolation = true;
                            idx += 2;
                            break;
                        }

                        literal.push(chars[idx]);
                        idx += 1;
                    }
                } else {
                    while idx < chars.len() {
                        let peek = chars[idx];

                        if peek == '"' {
                            terminated = true;
                            idx += 1;
                            break;
                        }
                        if peek == '#' && chars.get(idx + 1) == Some(&'{') {
                            is_interpolation = true;
                            idx += 2;
                            break;
                        }

                        literal.push(peek);
                        idx += 1;
                    }
                }

                if !literal.is_empty() {
                    let end_idx = if is_interpolation {
                        idx - 2
                    } else if terminated && *is_heredoc {
                        idx - 3
                    } else if terminated {
                        idx - 1
                    } else {
                        idx
                    };
                    tokens.push(Token::with_lexeme(
                        TokenKind::StringPart,
                        literal,
                        Span::new(start, end_idx),
                    ));
                }

                if terminated {
                    tokens.push(Token::simple(
                        TokenKind::StringEnd,
                        Span::new(idx - if *is_heredoc { 3 } else { 1 }, idx),
                    ));
                    state_stack.pop();
                } else if is_interpolation {
                    tokens.push(Token::simple(
                        TokenKind::InterpolationStart,
                        Span::new(idx - 2, idx),
                    ));
                    state_stack.push(LexerState::Normal);
                    current_brace_depth += 1;
                } else {
                    return Err(LexerError::unterminated_string(Span::new(
                        start,
                        chars.len(),
                    )));
                }
            }
        }
    }

    tokens.push(Token::simple(
        TokenKind::Eof,
        Span::new(chars.len(), chars.len()),
    ));
    Ok(tokens)
}

fn keyword_kind(lexeme: &str) -> Option<TokenKind> {
    match lexeme {
        "defmodule" => Some(TokenKind::Defmodule),
        "def" => Some(TokenKind::Def),
        "defp" => Some(TokenKind::Defp),
        "do" => Some(TokenKind::Do),
        "end" => Some(TokenKind::End),
        "if" => Some(TokenKind::If),
        "unless" => Some(TokenKind::Unless),
        "case" => Some(TokenKind::Case),
        "cond" => Some(TokenKind::Cond),
        "with" => Some(TokenKind::With),
        "for" => Some(TokenKind::For),
        "fn" => Some(TokenKind::Fn),
        "else" => Some(TokenKind::Else),
        "try" => Some(TokenKind::Try),
        "rescue" => Some(TokenKind::Rescue),
        "catch" => Some(TokenKind::Catch),
        "after" => Some(TokenKind::After),
        "raise" => Some(TokenKind::Raise),
        "true" => Some(TokenKind::True),
        "false" => Some(TokenKind::False),
        "nil" => Some(TokenKind::Nil),
        "and" => Some(TokenKind::And),
        "or" => Some(TokenKind::Or),
        "not" => Some(TokenKind::Not),
        "in" => Some(TokenKind::In),
        "when" => Some(TokenKind::When),
        _ => None,
    }
}

fn is_ident_start(value: char) -> bool {
    value == '_' || value.is_ascii_alphabetic()
}

fn is_ident_continue(value: char) -> bool {
    is_ident_start(value) || value.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::{scan_tokens, Span};

    fn dump_labels(source: &str) -> Vec<String> {
        scan_tokens(source)
            .expect("scanner should tokenize fixture source")
            .into_iter()
            .map(|token| token.dump_label())
            .collect()
    }

    #[test]
    fn scan_tokens_handles_minimal_module_fixture() {
        let labels = dump_labels("defmodule Math do\n  def add(a, b) do\n    a + b\n  end\nend\n");

        assert_eq!(
            labels,
            [
                "DEFMODULE(defmodule)",
                "IDENT(Math)",
                "DO(do)",
                "DEF(def)",
                "IDENT(add)",
                "LPAREN",
                "IDENT(a)",
                "COMMA",
                "IDENT(b)",
                "RPAREN",
                "DO(do)",
                "IDENT(a)",
                "PLUS",
                "IDENT(b)",
                "END(end)",
                "END(end)",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_supports_identifiers_and_literals() {
        let labels = dump_labels("value 42 3.14 \"ok\"");

        assert_eq!(
            labels,
            [
                "IDENT(value)",
                "INT(42)",
                "FLOAT(3.14)",
                "STRING(ok)",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_supports_triple_quoted_heredoc_literals() {
        let labels = dump_labels("\"\"\"hello\nworld\"\"\"");

        assert_eq!(labels, ["STRING(hello\nworld)", "EOF"]);
    }

    #[test]
    fn scan_tokens_supports_atoms_and_operators() {
        let labels = dump_labels(":ok value |> wrap(:ok)\nfn arg -> arg end");

        assert_eq!(
            labels,
            [
                "ATOM(ok)",
                "IDENT(value)",
                "PIPE_GT",
                "IDENT(wrap)",
                "LPAREN",
                "ATOM(ok)",
                "RPAREN",
                "FN(fn)",
                "IDENT(arg)",
                "ARROW",
                "IDENT(arg)",
                "END(end)",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_supports_map_fat_arrow_without_regressing_case_arrows() {
        let labels = dump_labels("%{\"status\" => 200} case value do :ok -> 1 end");

        assert_eq!(
            labels,
            [
                "PERCENT",
                "LBRACE",
                "STRING(status)",
                "FAT_ARROW",
                "INT(200)",
                "RBRACE",
                "CASE(case)",
                "IDENT(value)",
                "DO(do)",
                "ATOM(ok)",
                "ARROW",
                "INT(1)",
                "END(end)",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_supports_pattern_delimiters() {
        let labels = dump_labels("{:ok, value} [head, _] %{}");

        assert_eq!(
            labels,
            [
                "LBRACE",
                "ATOM(ok)",
                "COMMA",
                "IDENT(value)",
                "RBRACE",
                "LBRACKET",
                "IDENT(head)",
                "COMMA",
                "IDENT(_)",
                "RBRACKET",
                "PERCENT",
                "LBRACE",
                "RBRACE",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_supports_collection_literal_key_syntax() {
        let labels = dump_labels("%{ok: 1} [done: 2]");

        assert_eq!(
            labels,
            [
                "PERCENT",
                "LBRACE",
                "IDENT(ok)",
                "COLON",
                "INT(1)",
                "RBRACE",
                "LBRACKET",
                "IDENT(done)",
                "COLON",
                "INT(2)",
                "RBRACKET",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_supports_module_qualified_calls() {
        let labels = dump_labels("Math.helper()");

        assert_eq!(
            labels,
            [
                "IDENT(Math)",
                "DOT",
                "IDENT(helper)",
                "LPAREN",
                "RPAREN",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_supports_question_operator() {
        let labels = dump_labels("value()?");

        assert_eq!(
            labels,
            ["IDENT(value)", "LPAREN", "RPAREN", "QUESTION", "EOF",]
        );
    }

    #[test]
    fn scan_tokens_supports_pin_guards_and_match_operator() {
        let labels = dump_labels("[^value, tail] when tail == 8 -> value = tail");

        assert_eq!(
            labels,
            [
                "LBRACKET",
                "CARET",
                "IDENT(value)",
                "COMMA",
                "IDENT(tail)",
                "RBRACKET",
                "WHEN(when)",
                "IDENT(tail)",
                "EQ_EQ",
                "INT(8)",
                "ARROW",
                "IDENT(value)",
                "MATCH_EQ",
                "IDENT(tail)",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_supports_control_form_keywords_and_with_operator() {
        let labels = dump_labels("if value do 1 else 0 end unless value do 1 end cond do true -> 1 end with x <- 1 do x end for x <- list(1, 2) do x end");

        assert_eq!(
            labels,
            [
                "IF(if)",
                "IDENT(value)",
                "DO(do)",
                "INT(1)",
                "ELSE(else)",
                "INT(0)",
                "END(end)",
                "UNLESS(unless)",
                "IDENT(value)",
                "DO(do)",
                "INT(1)",
                "END(end)",
                "COND(cond)",
                "DO(do)",
                "TRUE(true)",
                "ARROW",
                "INT(1)",
                "END(end)",
                "WITH(with)",
                "IDENT(x)",
                "LEFT_ARROW",
                "INT(1)",
                "DO(do)",
                "IDENT(x)",
                "END(end)",
                "FOR(for)",
                "IDENT(x)",
                "LEFT_ARROW",
                "IDENT(list)",
                "LPAREN",
                "INT(1)",
                "COMMA",
                "INT(2)",
                "RPAREN",
                "DO(do)",
                "IDENT(x)",
                "END(end)",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_supports_defp_and_default_argument_operator() {
        let labels = dump_labels("defp add(value, inc \\\\ 2) do value + inc end");

        assert_eq!(
            labels,
            [
                "DEFP(defp)",
                "IDENT(add)",
                "LPAREN",
                "IDENT(value)",
                "COMMA",
                "IDENT(inc)",
                "BACKSLASH_BACKSLASH",
                "INT(2)",
                "RPAREN",
                "DO(do)",
                "IDENT(value)",
                "PLUS",
                "IDENT(inc)",
                "END(end)",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_supports_capture_and_function_value_invocation() {
        let labels = dump_labels("&(&1 + 1) (&Math.add/2).(2, 3); fun.(2)");

        assert_eq!(
            labels,
            [
                "AMPERSAND",
                "LPAREN",
                "AMPERSAND",
                "INT(1)",
                "PLUS",
                "INT(1)",
                "RPAREN",
                "LPAREN",
                "AMPERSAND",
                "IDENT(Math)",
                "DOT",
                "IDENT(add)",
                "SLASH",
                "INT(2)",
                "RPAREN",
                "DOT",
                "LPAREN",
                "INT(2)",
                "COMMA",
                "INT(3)",
                "RPAREN",
                "SEMICOLON",
                "IDENT(fun)",
                "DOT",
                "LPAREN",
                "INT(2)",
                "RPAREN",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_supports_module_attributes_and_forms() {
        let labels = dump_labels("@doc \"ok\" alias Math, as: M");

        assert_eq!(
            labels,
            [
                "AT",
                "IDENT(doc)",
                "STRING(ok)",
                "IDENT(alias)",
                "IDENT(Math)",
                "COMMA",
                "IDENT(as)",
                "COLON",
                "IDENT(M)",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_assigns_spans_for_tokens_and_eof() {
        let tokens = scan_tokens("def value").expect("scanner should tokenize fixture source");

        assert_eq!(tokens[0].span(), Span::new(0, 3));
        assert_eq!(tokens[1].span(), Span::new(4, 9));
        assert_eq!(tokens[2].span(), Span::new(9, 9));
    }

    #[test]
    fn scan_tokens_reports_invalid_character() {
        let error = scan_tokens("$").expect_err("scanner should reject unsupported characters");

        assert_eq!(error.to_string(), "invalid token '$' at offset 0");
        assert_eq!(error.span(), Span::new(0, 1));
    }

    #[test]
    fn scan_tokens_skips_hash_comments() {
        let labels = dump_labels("1 # trailing comment\n2");

        assert_eq!(labels, ["INT(1)", "INT(2)", "EOF"]);
    }

    #[test]
    fn scan_tokens_supports_basic_sigils_as_string_literals() {
        let labels = dump_labels("~s(hello) ~r/world/");

        assert_eq!(labels, ["STRING(hello)", "STRING(world)", "EOF"]);
    }

    #[test]
    fn scan_tokens_reports_unterminated_string_with_span() {
        let error =
            scan_tokens("\"oops").expect_err("scanner should reject unterminated string literals");

        assert_eq!(error.to_string(), "unterminated string literal at offset 0");
        assert_eq!(error.span(), Span::new(0, 5));
    }

    #[test]
    fn scan_tokens_supports_string_interpolation() {
        let labels = dump_labels("\"hello #{1 + 2} world\"");

        assert_eq!(
            labels,
            [
                "STRING_START",
                "STRING_PART(hello )",
                "INTERPOLATION_START",
                "INT(1)",
                "PLUS",
                "INT(2)",
                "INTERPOLATION_END",
                "STRING_PART( world)",
                "STRING_END",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_emits_lt_lt_for_double_angle_open() {
        let labels = dump_labels("<<");
        assert_eq!(labels, ["LT_LT", "EOF"]);
    }

    #[test]
    fn scan_tokens_emits_gt_gt_for_double_angle_close() {
        let labels = dump_labels(">>");
        assert_eq!(labels, ["GT_GT", "EOF"]);
    }

    #[test]
    fn scan_tokens_emits_lt_lt_lt_for_triple_angle_open() {
        let labels = dump_labels("<<<");
        assert_eq!(labels, ["LT_LT_LT", "EOF"]);
    }

    #[test]
    fn scan_tokens_distinguishes_lt_lt_from_lt_lt_lt() {
        let labels = dump_labels("<< <<<");
        assert_eq!(labels, ["LT_LT", "LT_LT_LT", "EOF"]);
    }

    #[test]
    fn scan_tokens_emits_gt_gt_gt_for_triple_angle_close() {
        let labels = dump_labels(">>>");
        assert_eq!(labels, ["GT_GT_GT", "EOF"]);
    }

    #[test]
    fn scan_tokens_distinguishes_gt_gt_from_gt_gt_gt() {
        let labels = dump_labels(">> >>>");
        assert_eq!(labels, ["GT_GT", "GT_GT_GT", "EOF"]);
    }

    #[test]
    fn scan_tokens_tokenizes_bitstring_literal_sequence() {
        // <<72, 101, 108>> should tokenize as LT_LT INT COMMA INT COMMA INT GT_GT
        let labels = dump_labels("<<72, 101, 108>>");
        assert_eq!(
            labels,
            ["LT_LT", "INT(72)", "COMMA", "INT(101)", "COMMA", "INT(108)", "GT_GT", "EOF",]
        );
    }

    #[test]
    fn scan_tokens_tokenizes_bitstring_with_size_annotation() {
        let labels = dump_labels("<<a::8, b::16>>");
        assert_eq!(
            labels,
            [
                "LT_LT", "IDENT(a)", "COLON", "COLON", "INT(8)", "COMMA", "IDENT(b)", "COLON",
                "COLON", "INT(16)", "GT_GT", "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_w_sigil_with_parens_produces_list() {
        let labels = dump_labels("~w(foo bar baz)");
        assert_eq!(
            labels,
            [
                "LBRACKET",
                "STRING(foo)",
                "COMMA",
                "STRING(bar)",
                "COMMA",
                "STRING(baz)",
                "RBRACKET",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_w_sigil_with_atom_modifier_produces_atom_list() {
        let labels = dump_labels("~w(ok error)a");
        assert_eq!(
            labels,
            [
                "LBRACKET",
                "ATOM(ok)",
                "COMMA",
                "ATOM(error)",
                "RBRACKET",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_w_sigil_with_single_word_produces_single_element_list() {
        let labels = dump_labels("~w(hello)");
        assert_eq!(labels, ["LBRACKET", "STRING(hello)", "RBRACKET", "EOF"]);
    }

    #[test]
    fn scan_tokens_char_literal_ascii_letter() {
        let labels = dump_labels("?a");
        assert_eq!(labels, ["INT(97)", "EOF"]);
    }

    #[test]
    fn scan_tokens_char_literal_newline_escape() {
        // ?\n is codepoint 10
        let labels = dump_labels("?\\n");
        assert_eq!(labels, ["INT(10)", "EOF"]);
    }

    #[test]
    fn scan_tokens_integer_with_underscores_multiple_groups() {
        let labels = dump_labels("1_000_000");
        assert_eq!(labels, ["INT(1000000)", "EOF"]);
    }

    #[test]
    fn scan_tokens_hex_literal_lowercase() {
        let labels = dump_labels("0xff");
        assert_eq!(labels, ["INT(255)", "EOF"]);
    }

    #[test]
    fn scan_tokens_octal_literal_lowercase() {
        let labels = dump_labels("0o77");
        assert_eq!(labels, ["INT(63)", "EOF"]);
    }

    #[test]
    fn scan_tokens_binary_literal_lowercase() {
        let labels = dump_labels("0b1010");
        assert_eq!(labels, ["INT(10)", "EOF"]);
    }

    #[test]
    fn scan_tokens_strict_equality_operators() {
        let labels = dump_labels("a === b !== c");
        assert_eq!(
            labels,
            [
                "IDENT(a)",
                "STRICT_EQ",
                "IDENT(b)",
                "STRICT_BANG_EQ",
                "IDENT(c)",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_bitwise_operators() {
        let labels = dump_labels("a &&& b ||| c ^^^ d ~~~ e");
        assert_eq!(
            labels,
            [
                "IDENT(a)",
                "AMP_AMP_AMP",
                "IDENT(b)",
                "PIPE_PIPE_PIPE",
                "IDENT(c)",
                "CARET_CARET_CARET",
                "IDENT(d)",
                "TILDE_TILDE_TILDE",
                "IDENT(e)",
                "EOF",
            ]
        );
    }

    #[test]
    fn scan_tokens_bitwise_shifts() {
        let labels = dump_labels("a <<< b >>> c");
        assert_eq!(
            labels,
            ["IDENT(a)", "LT_LT_LT", "IDENT(b)", "GT_GT_GT", "IDENT(c)", "EOF",]
        );
    }

    #[test]
    fn scan_tokens_stepped_range_slash_slash() {
        let labels = dump_labels("1..10//2");
        assert_eq!(
            labels,
            [
                "INT(1)",
                "DOT_DOT",
                "INT(10)",
                "SLASH_SLASH",
                "INT(2)",
                "EOF",
            ]
        );
    }

    // --- Numeric literal completeness tests ---

    #[test]
    fn scan_tokens_hex_literal_uppercase_prefix() {
        let labels = dump_labels("0XFF");
        assert_eq!(labels, ["INT(255)", "EOF"]);
    }

    #[test]
    fn scan_tokens_octal_literal_uppercase_prefix() {
        let labels = dump_labels("0O77");
        assert_eq!(labels, ["INT(63)", "EOF"]);
    }

    #[test]
    fn scan_tokens_binary_literal_uppercase_prefix() {
        let labels = dump_labels("0B1010");
        assert_eq!(labels, ["INT(10)", "EOF"]);
    }

    #[test]
    fn scan_tokens_hex_with_underscores() {
        let labels = dump_labels("0xFF_FF");
        assert_eq!(labels, ["INT(65535)", "EOF"]);
    }

    #[test]
    fn scan_tokens_binary_with_underscores() {
        let labels = dump_labels("0b1010_1010");
        assert_eq!(labels, ["INT(170)", "EOF"]);
    }

    #[test]
    fn scan_tokens_float_with_underscores() {
        let labels = dump_labels("1_000.50");
        assert_eq!(labels, ["FLOAT(1000.50)", "EOF"]);
    }

    #[test]
    fn scan_tokens_char_literal_space_is_question_operator() {
        // ?<space> should be Question token (space is a separator, not char literal)
        let labels = dump_labels("x? y");
        assert_eq!(labels, ["IDENT(x)", "QUESTION", "IDENT(y)", "EOF"]);
    }

    #[test]
    fn scan_tokens_char_literal_digit() {
        // ?0 should be INTEGER(48)
        let labels = dump_labels("?0");
        assert_eq!(labels, ["INT(48)", "EOF"]);
    }

    // --- Error cases ---

    #[test]
    fn scan_tokens_rejects_hex_with_no_digits() {
        let err = scan_tokens("0x").expect_err("0x with no digits should fail");
        assert!(
            err.to_string().contains("no digits"),
            "expected 'no digits' in error: {err}"
        );
    }

    #[test]
    fn scan_tokens_rejects_octal_with_no_digits() {
        let err = scan_tokens("0o").expect_err("0o with no digits should fail");
        assert!(
            err.to_string().contains("no digits"),
            "expected 'no digits' in error: {err}"
        );
    }

    #[test]
    fn scan_tokens_rejects_binary_with_no_digits() {
        let err = scan_tokens("0b").expect_err("0b with no digits should fail");
        assert!(
            err.to_string().contains("no digits"),
            "expected 'no digits' in error: {err}"
        );
    }

    #[test]
    fn scan_tokens_rejects_binary_invalid_digit() {
        let err = scan_tokens("0b12").expect_err("0b12 should fail — 2 is not a binary digit");
        assert!(
            err.to_string().contains("binary"),
            "expected 'binary' in error: {err}"
        );
    }

    #[test]
    fn scan_tokens_rejects_octal_invalid_digit() {
        let err = scan_tokens("0o78").expect_err("0o78 should fail — 8 is not an octal digit");
        assert!(
            err.to_string().contains("octal"),
            "expected 'octal' in error: {err}"
        );
    }

    #[test]
    fn scan_tokens_rejects_hex_separator_at_start() {
        let err = scan_tokens("0x_FF").expect_err("0x_FF should fail — separator at start");
        assert!(
            err.to_string().contains("separator"),
            "expected 'separator' in error: {err}"
        );
    }

    #[test]
    fn scan_tokens_rejects_hex_separator_at_end() {
        let err = scan_tokens("0xFF_").expect_err("0xFF_ should fail — separator at end");
        assert!(
            err.to_string().contains("separator"),
            "expected 'separator' in error: {err}"
        );
    }

    #[test]
    fn scan_tokens_rejects_decimal_separator_at_end() {
        let err = scan_tokens("100_").expect_err("100_ should fail — separator at end");
        assert!(
            err.to_string().contains("separator"),
            "expected 'separator' in error: {err}"
        );
    }

    // End of lexer unit tests.
}
