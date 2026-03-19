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
    pub(super) fn simple(kind: TokenKind, span: Span) -> Self {
        Self {
            kind,
            lexeme: String::new(),
            span,
        }
    }

    pub(super) fn with_lexeme(kind: TokenKind, lexeme: impl Into<String>, span: Span) -> Self {
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
    pub(super) kind: LexerErrorKind,
    pub(super) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LexerErrorKind {
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
    pub(super) fn invalid_token(value: char, span: Span) -> Self {
        Self {
            kind: LexerErrorKind::InvalidToken(value),
            span,
        }
    }

    pub(super) fn unterminated_string(span: Span) -> Self {
        Self {
            kind: LexerErrorKind::UnterminatedString,
            span,
        }
    }

    pub(super) fn empty_numeric_literal(prefix: &'static str, span: Span) -> Self {
        Self {
            kind: LexerErrorKind::EmptyNumericLiteral { prefix },
            span,
        }
    }

    pub(super) fn invalid_digit_for_base(digit: char, base: &'static str, span: Span) -> Self {
        Self {
            kind: LexerErrorKind::InvalidDigitForBase { digit, base },
            span,
        }
    }

    pub(super) fn misplaced_numeric_separator(span: Span) -> Self {
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
