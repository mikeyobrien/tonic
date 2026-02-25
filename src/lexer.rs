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
    OrOr,
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

    pub fn dump_label(&self) -> String {
        match self.kind {
            TokenKind::Defmodule => format!("DEFMODULE({})", self.lexeme),
            TokenKind::Def => format!("DEF({})", self.lexeme),
            TokenKind::Defp => format!("DEFP({})", self.lexeme),
            TokenKind::Do => format!("DO({})", self.lexeme),
            TokenKind::End => format!("END({})", self.lexeme),
            TokenKind::If => format!("IF({})", self.lexeme),
            TokenKind::Unless => format!("UNLESS({})", self.lexeme),
            TokenKind::Case => format!("CASE({})", self.lexeme),
            TokenKind::Cond => format!("COND({})", self.lexeme),
            TokenKind::With => format!("WITH({})", self.lexeme),
            TokenKind::For => format!("FOR({})", self.lexeme),
            TokenKind::Fn => format!("FN({})", self.lexeme),
            TokenKind::Else => format!("ELSE({})", self.lexeme),
            TokenKind::Try => format!("TRY({})", self.lexeme),
            TokenKind::Rescue => format!("RESCUE({})", self.lexeme),
            TokenKind::Catch => format!("CATCH({})", self.lexeme),
            TokenKind::After => format!("AFTER({})", self.lexeme),
            TokenKind::Raise => format!("RAISE({})", self.lexeme),
            TokenKind::True => format!("TRUE({})", self.lexeme),
            TokenKind::False => format!("FALSE({})", self.lexeme),
            TokenKind::Nil => format!("NIL({})", self.lexeme),
            TokenKind::And => format!("AND({})", self.lexeme),
            TokenKind::Or => format!("OR({})", self.lexeme),
            TokenKind::Not => format!("NOT({})", self.lexeme),
            TokenKind::In => format!("IN({})", self.lexeme),
            TokenKind::When => format!("WHEN({})", self.lexeme),
            TokenKind::Ident => format!("IDENT({})", self.lexeme),
            TokenKind::Atom => format!("ATOM({})", self.lexeme),
            TokenKind::Integer => format!("INT({})", self.lexeme),
            TokenKind::Float => format!("FLOAT({})", self.lexeme),
            TokenKind::String => format!("STRING({})", self.lexeme),
            TokenKind::StringStart => "STRING_START".to_string(),
            TokenKind::StringPart => format!("STRING_PART({})", self.lexeme),
            TokenKind::InterpolationStart => "INTERPOLATION_START".to_string(),
            TokenKind::InterpolationEnd => "INTERPOLATION_END".to_string(),
            TokenKind::StringEnd => "STRING_END".to_string(),
            TokenKind::LParen => "LPAREN".to_string(),
            TokenKind::RParen => "RPAREN".to_string(),
            TokenKind::LBrace => "LBRACE".to_string(),
            TokenKind::RBrace => "RBRACE".to_string(),
            TokenKind::LBracket => "LBRACKET".to_string(),
            TokenKind::RBracket => "RBRACKET".to_string(),
            TokenKind::Percent => "PERCENT".to_string(),
            TokenKind::At => "AT".to_string(),
            TokenKind::Colon => "COLON".to_string(),
            TokenKind::Comma => "COMMA".to_string(),
            TokenKind::Semicolon => "SEMICOLON".to_string(),
            TokenKind::Dot => "DOT".to_string(),
            TokenKind::DotDot => "DOT_DOT".to_string(),
            TokenKind::Caret => "CARET".to_string(),
            TokenKind::Plus => "PLUS".to_string(),
            TokenKind::PlusPlus => "PLUS_PLUS".to_string(),
            TokenKind::Minus => "MINUS".to_string(),
            TokenKind::MinusMinus => "MINUS_MINUS".to_string(),
            TokenKind::Star => "STAR".to_string(),
            TokenKind::Slash => "SLASH".to_string(),
            TokenKind::MatchEq => "MATCH_EQ".to_string(),
            TokenKind::EqEq => "EQ_EQ".to_string(),
            TokenKind::BangEq => "BANG_EQ".to_string(),
            TokenKind::Bang => "BANG".to_string(),
            TokenKind::Lt => "LT".to_string(),
            TokenKind::LtEq => "LT_EQ".to_string(),
            TokenKind::Gt => "GT".to_string(),
            TokenKind::GtEq => "GT_EQ".to_string(),
            TokenKind::LessGreater => "LESS_GREATER".to_string(),
            TokenKind::Question => "QUESTION".to_string(),
            TokenKind::Pipe => "PIPE".to_string(),
            TokenKind::PipeGt => "PIPE_GT".to_string(),
            TokenKind::FatArrow => "FAT_ARROW".to_string(),
            TokenKind::Arrow => "ARROW".to_string(),
            TokenKind::LeftArrow => "LEFT_ARROW".to_string(),
            TokenKind::BackslashBackslash => "BACKSLASH_BACKSLASH".to_string(),
            TokenKind::Ampersand => "AMPERSAND".to_string(),
            TokenKind::AndAnd => "AND_AND".to_string(),
            TokenKind::OrOr => "OR_OR".to_string(),
            TokenKind::Eof => "EOF".to_string(),
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

    #[cfg(test)]
    pub fn span(&self) -> Span {
        self.span
    }
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
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
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::Caret, Span::new(start, idx)));
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
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::Slash, Span::new(start, idx)));
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
                        if chars.get(idx + 1) == Some(&'=') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::BangEq, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Bang, Span::new(start, idx)));
                        }
                    }
                    '<' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'=') {
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
                        if chars.get(idx + 1) == Some(&'=') {
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
                        tokens.push(Token::simple(TokenKind::Question, Span::new(start, idx)));
                    }
                    '|' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'>') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::PipeGt, Span::new(start, idx)));
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
                        if chars.get(idx + 1) == Some(&'&') {
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
                        let Some(sigil_kind) = chars.get(idx + 1).copied() else {
                            return Err(LexerError::invalid_token(
                                '~',
                                Span::new(start, start + 1),
                            ));
                        };

                        if !matches!(sigil_kind, 's' | 'r') {
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
                        tokens.push(Token::with_lexeme(
                            TokenKind::String,
                            lexeme,
                            Span::new(start, idx),
                        ));
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

                        while idx < chars.len() && chars[idx].is_ascii_digit() {
                            idx += 1;
                        }

                        let mut kind = TokenKind::Integer;
                        if idx + 1 < chars.len()
                            && chars[idx] == '.'
                            && chars[idx + 1].is_ascii_digit()
                        {
                            kind = TokenKind::Float;
                            idx += 1;

                            while idx < chars.len() && chars[idx].is_ascii_digit() {
                                idx += 1;
                            }
                        }

                        let lexeme: String = chars[start..idx].iter().collect();
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
}
