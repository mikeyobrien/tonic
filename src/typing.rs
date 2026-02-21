use crate::parser::{Ast, Expr};
use std::collections::{BTreeMap, HashMap};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSummary {
    signatures: BTreeMap<String, String>,
}

impl TypeSummary {
    pub fn signature(&self, name: &str) -> Option<&str> {
        self.signatures.get(name).map(String::as_str)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypingDiagnosticCode {
    TypeMismatch,
}

impl TypingDiagnosticCode {
    fn as_str(self) -> &'static str {
        match self {
            Self::TypeMismatch => "E2001",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypingError {
    code: Option<TypingDiagnosticCode>,
    message: String,
    offset: Option<usize>,
}

impl TypingError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            code: None,
            message: message.into(),
            offset: None,
        }
    }

    fn type_mismatch(expected: Type, found: Type, offset: Option<usize>) -> Self {
        Self {
            code: Some(TypingDiagnosticCode::TypeMismatch),
            message: format!(
                "type mismatch: expected {}, found {}",
                expected.label(),
                found.label()
            ),
            offset,
        }
    }
}

impl fmt::Display for TypingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.code, self.offset) {
            (Some(code), Some(offset)) => {
                write!(f, "[{}] {} at offset {offset}", code.as_str(), self.message)
            }
            (Some(code), None) => write!(f, "[{}] {}", code.as_str(), self.message),
            (None, _) => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for TypingError {}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Type {
    Int,
    Dynamic,
    Var(TypeVarId),
}

impl Type {
    fn label(&self) -> &'static str {
        match self {
            Type::Int => "int",
            Type::Dynamic | Type::Var(_) => "dynamic",
        }
    }
}

type TypeVarId = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionSignature {
    params: Vec<Type>,
    return_type: Type,
}

#[derive(Debug, Default)]
struct ConstraintSolver {
    next_var: TypeVarId,
    substitutions: HashMap<TypeVarId, Type>,
}

impl ConstraintSolver {
    fn fresh_var(&mut self) -> Type {
        let id = self.next_var;
        self.next_var += 1;
        Type::Var(id)
    }

    fn unify(
        &mut self,
        expected: Type,
        found: Type,
        offset: Option<usize>,
    ) -> Result<(), TypingError> {
        let expected = self.resolve(expected);
        let found = self.resolve(found);

        match (expected, found) {
            (Type::Var(expected_id), Type::Var(found_id)) if expected_id == found_id => Ok(()),
            (Type::Var(id), ty) => {
                self.substitutions.insert(id, ty);
                Ok(())
            }
            (expected_ty, Type::Var(id)) => {
                self.substitutions.insert(id, expected_ty);
                Ok(())
            }
            (Type::Int, Type::Int) | (Type::Dynamic, Type::Dynamic) => Ok(()),
            (expected_ty, found_ty) => {
                Err(TypingError::type_mismatch(expected_ty, found_ty, offset))
            }
        }
    }

    fn resolve(&mut self, ty: Type) -> Type {
        match ty {
            Type::Var(id) => {
                if let Some(mapped) = self.substitutions.get(&id).cloned() {
                    let resolved = self.resolve(mapped);
                    self.substitutions.insert(id, resolved.clone());
                    resolved
                } else {
                    Type::Var(id)
                }
            }
            other => other,
        }
    }

    fn finalize(&mut self, ty: Type) -> Type {
        match self.resolve(ty) {
            Type::Var(_) => Type::Dynamic,
            concrete => concrete,
        }
    }
}

pub fn infer_types(ast: &Ast) -> Result<TypeSummary, TypingError> {
    let mut solver = ConstraintSolver::default();
    let mut signatures: BTreeMap<String, FunctionSignature> = BTreeMap::new();

    for module in &ast.modules {
        for function in &module.functions {
            let params = function.params.iter().map(|_| solver.fresh_var()).collect();
            let return_type = solver.fresh_var();
            signatures.insert(
                qualify_function_name(&module.name, &function.name),
                FunctionSignature {
                    params,
                    return_type,
                },
            );
        }
    }

    for module in &ast.modules {
        for function in &module.functions {
            let function_name = qualify_function_name(&module.name, &function.name);
            let declared_return_type = signatures
                .get(&function_name)
                .expect("function signature should be pre-seeded")
                .return_type
                .clone();

            let inferred_body_type =
                infer_expression_type(&function.body, &module.name, &signatures, &mut solver)?;

            solver.unify(
                declared_return_type,
                inferred_body_type,
                Some(function.body.offset()),
            )?;
        }
    }

    let summary = signatures
        .into_iter()
        .map(|(name, signature)| {
            let params = signature
                .params
                .into_iter()
                .map(|param| solver.finalize(param))
                .collect::<Vec<_>>();
            let return_type = solver.finalize(signature.return_type);
            (name, format_signature(&params, &return_type))
        })
        .collect();

    Ok(TypeSummary {
        signatures: summary,
    })
}

fn infer_expression_type(
    expr: &Expr,
    current_module: &str,
    signatures: &BTreeMap<String, FunctionSignature>,
    solver: &mut ConstraintSolver,
) -> Result<Type, TypingError> {
    match expr {
        Expr::Int { .. } => Ok(Type::Int),
        Expr::Call { callee, args, .. } => {
            for arg in args {
                infer_expression_type(arg, current_module, signatures, solver)?;
            }

            let target_name = qualify_call_target(current_module, callee);
            let signature = signatures.get(&target_name).ok_or_else(|| {
                TypingError::new(format!(
                    "unknown call target during inference: {target_name}"
                ))
            })?;

            if signature.params.len() != args.len() {
                return Err(TypingError::new(format!(
                    "arity mismatch for {target_name}: expected {} args, found {}",
                    signature.params.len(),
                    args.len()
                )));
            }

            Ok(signature.return_type.clone())
        }
        Expr::Binary { left, right, .. } => {
            let left_type = infer_expression_type(left, current_module, signatures, solver)?;
            let right_type = infer_expression_type(right, current_module, signatures, solver)?;

            solver.unify(Type::Int, left_type, Some(left.offset()))?;
            solver.unify(Type::Int, right_type, Some(right.offset()))?;

            Ok(Type::Int)
        }
        Expr::Pipe { left, right, .. } => {
            infer_expression_type(left, current_module, signatures, solver)?;
            infer_expression_type(right, current_module, signatures, solver)
        }
        Expr::Case {
            subject, branches, ..
        } => {
            infer_expression_type(subject, current_module, signatures, solver)?;

            let mut inferred_case_type = None;

            for branch in branches {
                let branch_type =
                    infer_expression_type(branch.body(), current_module, signatures, solver)?;

                if let Some(existing) = inferred_case_type.clone() {
                    solver.unify(existing, branch_type, Some(branch.body().offset()))?;
                } else {
                    inferred_case_type = Some(branch_type);
                }
            }

            Ok(inferred_case_type.unwrap_or(Type::Dynamic))
        }
    }
}

fn qualify_function_name(module_name: &str, function_name: &str) -> String {
    format!("{module_name}.{function_name}")
}

fn qualify_call_target(current_module: &str, callee: &str) -> String {
    if callee.contains('.') {
        callee.to_string()
    } else {
        qualify_function_name(current_module, callee)
    }
}

fn format_signature(params: &[Type], return_type: &Type) -> String {
    let params = params
        .iter()
        .map(Type::label)
        .collect::<Vec<_>>()
        .join(", ");

    format!("fn({params}) -> {}", return_type.label())
}

#[cfg(test)]
mod tests {
    use super::infer_types;
    use crate::lexer::scan_tokens;
    use crate::parser::parse_ast;

    #[test]
    fn infer_types_supports_polymorphic_like_helper_with_concrete_call_sites() {
        let source = "defmodule Demo do\n  def helper(value) do\n    1\n  end\n\n  def one() do\n    1\n  end\n\n  def run() do\n    helper(1) + helper(one())\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize typing fixture");
        let ast = parse_ast(&tokens).expect("parser should build typing fixture ast");

        let summary = infer_types(&ast)
            .expect("type inference should succeed for helper reuse across call sites");

        assert_eq!(summary.signature("Demo.helper"), Some("fn(dynamic) -> int"));
        assert_eq!(summary.signature("Demo.run"), Some("fn() -> int"));
    }

    #[test]
    fn infer_types_reports_type_mismatch_with_span_offset() {
        let source = "defmodule Demo do\n  def unknown() do\n    case source() do\n    end\n  end\n\n  def source() do\n    1\n  end\n\n  def run() do\n    unknown() + 1\n  end\nend\n";
        let tokens = scan_tokens(source).expect("scanner should tokenize mismatch fixture");
        let ast = parse_ast(&tokens).expect("parser should build mismatch fixture ast");

        let error = infer_types(&ast)
            .expect_err("type inference should reject implicit dynamic-to-int coercion");

        assert_eq!(
            error.to_string(),
            "[E2001] type mismatch: expected int, found dynamic at offset 123"
        );
    }
}
