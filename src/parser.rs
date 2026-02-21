use crate::lexer::{Span, Token, TokenKind};
use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Ast {
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(String);

impl NodeId {
    fn new(kind: &'static str, value: u64) -> Self {
        Self(format!("{kind}-{value:04}"))
    }
}

#[derive(Debug, Default)]
struct NodeIdGenerator {
    next: u64,
}

impl NodeIdGenerator {
    fn next_module(&mut self) -> NodeId {
        self.next("module")
    }

    fn next_function(&mut self) -> NodeId {
        self.next("function")
    }

    fn next_expr(&mut self) -> NodeId {
        self.next("expr")
    }

    fn next(&mut self, kind: &'static str) -> NodeId {
        self.next += 1;
        NodeId::new(kind, self.next)
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Module {
    #[serde(skip_serializing)]
    pub id: NodeId,
    pub name: String,
    pub functions: Vec<Function>,
}

impl Module {
    fn with_id(id: NodeId, name: String, functions: Vec<Function>) -> Self {
        Self {
            id,
            name,
            functions,
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Function {
    #[serde(skip_serializing)]
    pub id: NodeId,
    pub name: String,
    pub params: Vec<String>,
    pub body: Expr,
}

impl Function {
    fn with_id(id: NodeId, name: String, params: Vec<String>, body: Expr) -> Self {
        Self {
            id,
            name,
            params,
            body,
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Expr {
    Int {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        value: i64,
    },
    Call {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        callee: String,
        args: Vec<Expr>,
    },
    Binary {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Pipe {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Case {
        #[serde(skip_serializing)]
        id: NodeId,
        #[serde(skip_serializing)]
        offset: usize,
        subject: Box<Expr>,
        branches: Vec<CaseBranch>,
    },
}

pub type CaseBranch = Branch<Pattern>;

#[derive(Debug, PartialEq, Eq)]
pub struct Branch<Head>
where
    Head: BranchHead,
{
    head: Head,
    body: Expr,
}

impl<Head> Branch<Head>
where
    Head: BranchHead,
{
    fn new(head: Head, body: Expr) -> Self {
        Self { head, body }
    }

    pub fn head(&self) -> &Head {
        &self.head
    }

    pub fn body(&self) -> &Expr {
        &self.body
    }
}

impl<Head> Serialize for Branch<Head>
where
    Head: BranchHead,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut branch = serializer.serialize_struct("Branch", 2)?;
        branch.serialize_field(Head::FIELD_NAME, self.head())?;
        branch.serialize_field("body", self.body())?;
        branch.end()
    }
}

pub trait BranchHead: Serialize {
    const FIELD_NAME: &'static str;
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Pattern {
    Atom { value: String },
    Bind { name: String },
    Wildcard,
    Tuple { items: Vec<Pattern> },
    List { items: Vec<Pattern> },
    Map { entries: Vec<MapPatternEntry> },
}

impl BranchHead for Pattern {
    const FIELD_NAME: &'static str = "pattern";
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct MapPatternEntry {
    key: Pattern,
    value: Pattern,
}

impl Expr {
    fn int(id: NodeId, offset: usize, value: i64) -> Self {
        Self::Int { id, offset, value }
    }

    fn call(id: NodeId, offset: usize, callee: String, args: Vec<Expr>) -> Self {
        Self::Call {
            id,
            offset,
            callee,
            args,
        }
    }

    fn binary(id: NodeId, op: BinaryOp, left: Expr, right: Expr) -> Self {
        let offset = left.offset();

        Self::Binary {
            id,
            offset,
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    fn pipe(id: NodeId, left: Expr, right: Expr) -> Self {
        let offset = left.offset();

        Self::Pipe {
            id,
            offset,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    fn case(id: NodeId, offset: usize, subject: Expr, branches: Vec<CaseBranch>) -> Self {
        Self::Case {
            id,
            offset,
            subject: Box::new(subject),
            branches,
        }
    }

    pub fn offset(&self) -> usize {
        match self {
            Self::Int { offset, .. }
            | Self::Call { offset, .. }
            | Self::Binary { offset, .. }
            | Self::Pipe { offset, .. }
            | Self::Case { offset, .. } => *offset,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BinaryOp {
    Plus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserError {
    message: String,
    span: Option<Span>,
}

impl ParserError {
    fn at_current(message: impl Into<String>, token: Option<&Token>) -> Self {
        Self {
            message: message.into(),
            span: token.map(Token::span),
        }
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(span) = self.span {
            write!(f, "{} at offset {}", self.message, span.start())
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for ParserError {}

pub fn parse_ast(tokens: &[Token]) -> Result<Ast, ParserError> {
    Parser::new(tokens).parse_program()
}

struct Parser<'a> {
    tokens: &'a [Token],
    index: usize,
    node_ids: NodeIdGenerator,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            index: 0,
            node_ids: NodeIdGenerator::default(),
        }
    }

    fn parse_program(mut self) -> Result<Ast, ParserError> {
        let mut modules = Vec::new();

        while !self.is_at_end() {
            modules.push(self.parse_module()?);
        }

        Ok(Ast { modules })
    }

    fn parse_module(&mut self) -> Result<Module, ParserError> {
        let id = self.node_ids.next_module();

        self.expect(TokenKind::Defmodule, "defmodule")?;
        let name = self.expect_ident("module name")?;
        self.expect(TokenKind::Do, "do")?;

        let mut functions = Vec::new();
        while !self.check(TokenKind::End) {
            if self.is_at_end() {
                return Err(self.expected("function declaration"));
            }

            functions.push(self.parse_function()?);
        }

        self.expect(TokenKind::End, "end")?;

        Ok(Module::with_id(id, name, functions))
    }

    fn parse_function(&mut self) -> Result<Function, ParserError> {
        let id = self.node_ids.next_function();

        self.expect(TokenKind::Def, "def")?;
        let name = self.expect_ident("function name")?;
        self.expect(TokenKind::LParen, "(")?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen, ")")?;
        self.expect(TokenKind::Do, "do")?;
        let body = self.parse_expression()?;
        self.expect(TokenKind::End, "end")?;

        Ok(Function::with_id(id, name, params, body))
    }

    fn parse_params(&mut self) -> Result<Vec<String>, ParserError> {
        let mut params = Vec::new();

        if self.check(TokenKind::RParen) {
            return Ok(params);
        }

        loop {
            params.push(self.expect_ident("parameter name")?);

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        Ok(params)
    }

    fn parse_expression(&mut self) -> Result<Expr, ParserError> {
        self.parse_pipe_expression()
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
        let mut left = self.parse_primary_expression()?;

        while let Some((precedence, op)) = self.current_binary_operator() {
            if precedence < min_precedence {
                break;
            }

            self.advance();
            let right = self.parse_binary_expression(precedence + 1)?;
            left = Expr::binary(self.node_ids.next_expr(), op, left, right);
        }

        Ok(left)
    }

    fn parse_primary_expression(&mut self) -> Result<Expr, ParserError> {
        if self.check(TokenKind::Case) {
            return self.parse_case_expression();
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

        if self.check(TokenKind::Ident) {
            let callee_token = self
                .advance()
                .expect("identifier token should be available");
            let offset = callee_token.span().start();
            let mut callee = callee_token.lexeme().to_string();

            if self.match_kind(TokenKind::Dot) {
                let function_name = self.expect_ident("qualified function name")?;
                callee = format!("{callee}.{function_name}");
            }

            self.expect(TokenKind::LParen, "(")?;
            let args = self.parse_call_args()?;
            self.expect(TokenKind::RParen, ")")?;

            return Ok(Expr::call(self.node_ids.next_expr(), offset, callee, args));
        }

        Err(self.expected("expression"))
    }

    fn parse_case_expression(&mut self) -> Result<Expr, ParserError> {
        let offset = self.expect_token(TokenKind::Case, "case")?.span().start();
        let subject = self.parse_expression()?;
        self.expect(TokenKind::Do, "do")?;

        let mut branches = Vec::new();
        while !self.check(TokenKind::End) {
            if self.is_at_end() {
                return Err(self.expected("case branch"));
            }

            branches.push(self.parse_case_branch()?);
        }

        self.expect(TokenKind::End, "end")?;

        Ok(Expr::case(
            self.node_ids.next_expr(),
            offset,
            subject,
            branches,
        ))
    }

    fn parse_case_branch(&mut self) -> Result<CaseBranch, ParserError> {
        let pattern = self.parse_pattern()?;
        self.expect(TokenKind::Arrow, "->")?;
        let body = self.parse_expression()?;

        Ok(CaseBranch::new(pattern, body))
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParserError> {
        if self.match_kind(TokenKind::LBrace) {
            let items = self.parse_pattern_items(TokenKind::RBrace)?;
            return Ok(Pattern::Tuple { items });
        }

        if self.match_kind(TokenKind::LBracket) {
            let items = self.parse_pattern_items(TokenKind::RBracket)?;
            return Ok(Pattern::List { items });
        }

        if self.match_kind(TokenKind::Percent) {
            return self.parse_map_pattern();
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

    fn parse_pattern_items(&mut self, closing: TokenKind) -> Result<Vec<Pattern>, ParserError> {
        let mut items = Vec::new();

        if self.check(closing) {
            self.advance();
            return Ok(items);
        }

        loop {
            items.push(self.parse_pattern()?);

            if self.match_kind(TokenKind::Comma) {
                continue;
            }

            break;
        }

        self.expect(closing, "pattern terminator")?;
        Ok(items)
    }

    fn parse_map_pattern(&mut self) -> Result<Pattern, ParserError> {
        self.expect(TokenKind::LBrace, "{")?;

        let mut entries = Vec::new();
        if !self.check(TokenKind::RBrace) {
            loop {
                let key = self.parse_pattern()?;
                self.expect(TokenKind::Arrow, "->")?;
                let value = self.parse_pattern()?;
                entries.push(MapPatternEntry { key, value });

                if self.match_kind(TokenKind::Comma) {
                    continue;
                }

                break;
            }
        }

        self.expect(TokenKind::RBrace, "}")?;

        Ok(Pattern::Map { entries })
    }

    fn current_binary_operator(&self) -> Option<(u8, BinaryOp)> {
        self.current().and_then(|token| match token.kind() {
            TokenKind::Plus => Some((10, BinaryOp::Plus)),
            _ => None,
        })
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expr>, ParserError> {
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

    fn expect(&mut self, kind: TokenKind, expected: &str) -> Result<(), ParserError> {
        self.expect_token(kind, expected).map(|_| ())
    }

    fn expect_token(&mut self, kind: TokenKind, expected: &str) -> Result<&'a Token, ParserError> {
        if self.check(kind) {
            Ok(self.advance().expect("expected token should be available"))
        } else {
            Err(self.expected(expected))
        }
    }

    fn expect_ident(&mut self, expected: &str) -> Result<String, ParserError> {
        if !self.check(TokenKind::Ident) {
            return Err(self.expected(expected));
        }

        Ok(self
            .advance()
            .expect("identifier token should be available")
            .lexeme()
            .to_string())
    }

    fn expected(&self, expected: &str) -> ParserError {
        let found = self
            .current()
            .map(|token| token.dump_label())
            .unwrap_or_else(|| "EOF".to_string());

        ParserError::at_current(
            format!("expected {expected}, found {found}"),
            self.current(),
        )
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.current()
            .map(|token| token.kind() == kind)
            .unwrap_or(false)
    }

    fn match_kind(&mut self, kind: TokenKind) -> bool {
        if !self.check(kind) {
            return false;
        }

        self.index += 1;
        true
    }

    fn advance(&mut self) -> Option<&'a Token> {
        let token = self.current()?;
        self.index += 1;
        Some(token)
    }

    fn current(&self) -> Option<&'a Token> {
        self.tokens.get(self.index)
    }

    fn is_at_end(&self) -> bool {
        self.check(TokenKind::Eof)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_ast, Expr};
    use crate::lexer::scan_tokens;

    #[test]
    fn parse_ast_supports_single_module_with_two_functions() {
        let tokens = scan_tokens(
            "defmodule Math do\n  def one() do\n    1\n  end\n\n  def two() do\n    one()\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(ast.modules.len(), 1);
        assert_eq!(ast.modules[0].name, "Math");
        assert_eq!(ast.modules[0].functions.len(), 2);
        assert_eq!(ast.modules[0].functions[0].name, "one");
        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({"kind":"int","value":1})
        );
        assert_eq!(ast.modules[0].functions[1].name, "two");
        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[1].body)
                .expect("expression should serialize"),
            serde_json::json!({"kind":"call","callee":"one","args":[]})
        );
    }

    #[test]
    fn parse_ast_supports_nested_calls_with_plus_precedence() {
        let tokens = scan_tokens(
            "defmodule Math do\n  def compute() do\n    combine(1, 2) + wrap(inner(3 + 4))\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({
                "kind":"binary",
                "op":"plus",
                "left":{
                    "kind":"call",
                    "callee":"combine",
                    "args":[
                        {"kind":"int","value":1},
                        {"kind":"int","value":2}
                    ]
                },
                "right":{
                    "kind":"call",
                    "callee":"wrap",
                    "args":[
                        {
                            "kind":"call",
                            "callee":"inner",
                            "args":[
                                {
                                    "kind":"binary",
                                    "op":"plus",
                                    "left":{"kind":"int","value":3},
                                    "right":{"kind":"int","value":4}
                                }
                            ]
                        }
                    ]
                }
            })
        );
    }

    #[test]
    fn parse_ast_supports_module_qualified_calls() {
        let tokens =
            scan_tokens("defmodule Demo do\n  def run() do\n    Math.helper()\n  end\nend\n")
                .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({"kind":"call","callee":"Math.helper","args":[]})
        );
    }

    #[test]
    fn parse_ast_supports_case_patterns() {
        let tokens = scan_tokens(
            "defmodule PatternDemo do\n  def run() do\n    case input() do\n      {:ok, value} -> 1\n      [head, tail] -> 2\n      %{} -> 3\n      _ -> 4\n    end\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");

        assert_eq!(
            serde_json::to_value(&ast.modules[0].functions[0].body)
                .expect("expression should serialize"),
            serde_json::json!({
                "kind":"case",
                "subject":{"kind":"call","callee":"input","args":[]},
                "branches":[
                    {
                        "pattern":{
                            "kind":"tuple",
                            "items":[
                                {"kind":"atom","value":"ok"},
                                {"kind":"bind","name":"value"}
                            ]
                        },
                        "body":{"kind":"int","value":1}
                    },
                    {
                        "pattern":{
                            "kind":"list",
                            "items":[
                                {"kind":"bind","name":"head"},
                                {"kind":"bind","name":"tail"}
                            ]
                        },
                        "body":{"kind":"int","value":2}
                    },
                    {
                        "pattern":{"kind":"map","entries":[]},
                        "body":{"kind":"int","value":3}
                    },
                    {
                        "pattern":{"kind":"wildcard"},
                        "body":{"kind":"int","value":4}
                    }
                ]
            })
        );
    }

    #[test]
    fn parse_ast_exposes_normalized_case_branch_head_and_body() {
        let tokens = scan_tokens(
            "defmodule PatternDemo do\n  def run() do\n    case input() do\n      {:ok, value} -> 1\n      _ -> 2\n    end\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let ast = parse_ast(&tokens).expect("parser should produce ast");
        let Expr::Case { branches, .. } = &ast.modules[0].functions[0].body else {
            panic!("expected case expression body");
        };

        assert_eq!(branches.len(), 2);
        assert_eq!(
            serde_json::to_value(branches[0].head()).expect("branch head should serialize"),
            serde_json::json!({
                "kind":"tuple",
                "items":[
                    {"kind":"atom","value":"ok"},
                    {"kind":"bind","name":"value"}
                ]
            })
        );
        assert_eq!(
            serde_json::to_value(branches[0].body()).expect("branch body should serialize"),
            serde_json::json!({"kind":"int","value":1})
        );
    }

    #[test]
    fn parse_ast_assigns_stable_node_ids() {
        let tokens = scan_tokens(
            "defmodule Math do\n  def one() do\n    1\n  end\n\n  def two() do\n    one()\n  end\nend\n",
        )
        .expect("scanner should tokenize parser fixture");

        let first = parse_ast(&tokens).expect("parser should produce ast");
        let second = parse_ast(&tokens).expect("parser should produce ast");

        let first_ids = collect_node_ids(&first);
        let second_ids = collect_node_ids(&second);

        assert_eq!(
            first_ids,
            [
                "module-0001",
                "function-0002",
                "expr-0003",
                "function-0004",
                "expr-0005",
            ]
        );
        assert_eq!(first_ids, second_ids);

        let unique_count = first_ids
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();

        assert_eq!(unique_count, first_ids.len());
    }

    #[test]
    fn parse_ast_reports_missing_module_end() {
        let tokens = scan_tokens("defmodule Broken do\n  def one() do\n    1\n  end\n")
            .expect("scanner should tokenize parser fixture");

        let error = parse_ast(&tokens).expect_err("parser should reject missing end");

        assert!(
            error
                .to_string()
                .starts_with("expected function declaration, found EOF"),
            "unexpected parser error: {error}"
        );
    }

    fn collect_node_ids(ast: &super::Ast) -> Vec<String> {
        let mut ids = Vec::new();

        for module in &ast.modules {
            ids.push(module.id.0.clone());

            for function in &module.functions {
                ids.push(function.id.0.clone());
                collect_expr_ids(&function.body, &mut ids);
            }
        }

        ids
    }

    fn collect_expr_ids(expr: &Expr, ids: &mut Vec<String>) {
        match expr {
            Expr::Int { id, .. } => ids.push(id.0.clone()),
            Expr::Call { id, args, .. } => {
                ids.push(id.0.clone());

                for arg in args {
                    collect_expr_ids(arg, ids);
                }
            }
            Expr::Binary {
                id, left, right, ..
            } => {
                ids.push(id.0.clone());
                collect_expr_ids(left, ids);
                collect_expr_ids(right, ids);
            }
            Expr::Pipe {
                id, left, right, ..
            } => {
                ids.push(id.0.clone());
                collect_expr_ids(left, ids);
                collect_expr_ids(right, ids);
            }
            Expr::Case {
                id,
                subject,
                branches,
                ..
            } => {
                ids.push(id.0.clone());
                collect_expr_ids(subject, ids);

                for branch in branches {
                    collect_expr_ids(branch.body(), ids);
                }
            }
        }
    }
}
