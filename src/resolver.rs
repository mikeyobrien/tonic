use crate::guard_builtins;
use crate::parser::{Ast, Expr, ModuleForm, Pattern};
use crate::resolver_diag::ResolverError;
use std::collections::{HashMap, HashSet};

pub fn resolve_ast(ast: &Ast) -> Result<(), ResolverError> {
    ensure_no_duplicate_modules(ast)?;

    let module_graph = build_module_graph(ast);
    detect_cycles(&module_graph)?;

    for module in &ast.modules {
        let mut resolver = Resolver::new(ast);
        resolver.resolve_module(module)?;
    }
    Ok(())
}

fn ensure_no_duplicate_modules(ast: &Ast) -> Result<(), ResolverError> {
    let mut seen: HashMap<String, ()> = HashMap::new();
    for module in &ast.modules {
        let name = module.name.clone();
        if seen.contains_key(&name) {
            return Err(ResolverError::DuplicateModule(name));
        }
        seen.insert(name, ());
    }
    Ok(())
}

fn build_module_graph(ast: &Ast) -> HashMap<String, Vec<String>> {
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for module in &ast.modules {
        let deps: Vec<String> = module
            .imports
            .iter()
            .map(|i| i.module.clone())
            .collect();
        graph.insert(module.name.clone(), deps);
    }
    graph
}

fn detect_cycles(graph: &HashMap<String, Vec<String>>) -> Result<(), ResolverError> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut stack: HashSet<String> = HashSet::new();

    for node in graph.keys() {
        if !visited.contains(node) {
            dfs(node, graph, &mut visited, &mut stack)?;
        }
    }
    Ok(())
}

fn dfs(
    node: &str,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    stack: &mut HashSet<String>,
) -> Result<(), ResolverError> {
    visited.insert(node.to_string());
    stack.insert(node.to_string());

    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            if !visited.contains(neighbor) {
                dfs(neighbor, graph, visited, stack)?;
            } else if stack.contains(neighbor) {
                return Err(ResolverError::CyclicDependency(neighbor.to_string()));
            }
        }
    }

    stack.remove(node);
    Ok(())
}

pub struct Resolver<'a> {
    ast: &'a Ast,
    // module-level definitions visible to all functions in this module
    module_scope: HashMap<String, BindingKind>,
    // per-function scope stack
    scopes: Vec<HashMap<String, BindingKind>>,
    current_module: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BindingKind {
    Variable,
    Function(usize), // arity
    Module,
    Protocol,
    Impl,
}

impl<'a> Resolver<'a> {
    pub fn new(ast: &'a Ast) -> Self {
        Resolver {
            ast,
            module_scope: HashMap::new(),
            scopes: Vec::new(),
            current_module: None,
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define(&mut self, name: String, kind: BindingKind) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, kind);
        } else {
            self.module_scope.insert(name, kind);
        }
    }

    fn lookup(&self, name: &str) -> Option<&BindingKind> {
        for scope in self.scopes.iter().rev() {
            if let Some(k) = scope.get(name) {
                return Some(k);
            }
        }
        self.module_scope.get(name)
    }

    pub fn resolve_module(&mut self, module: &ModuleForm) -> Result<(), ResolverError> {
        self.current_module = Some(module.name.clone());
        self.module_scope.clear();
        self.scopes.clear();

        // Pre-populate module scope with all top-level names
        for item in &module.body {
            match item {
                Expr::Def(d) => {
                    self.module_scope
                        .insert(d.name.clone(), BindingKind::Function(d.args.len()));
                }
                Expr::DefMacro(dm) => {
                    self.module_scope
                        .insert(dm.name.clone(), BindingKind::Function(dm.args.len()));
                }
                Expr::Defstruct(ds) => {
                    self.module_scope
                        .insert(ds.name.clone(), BindingKind::Module);
                }
                Expr::Defprotocol(dp) => {
                    self.module_scope
                        .insert(dp.name.clone(), BindingKind::Protocol);
                }
                Expr::Defimpl(di) => {
                    let key = format!("{}::{}", di.protocol, di.target);
                    self.module_scope.insert(key, BindingKind::Impl);
                }
                _ => {}
            }
        }

        // Resolve imports
        for import in &module.imports {
            let mod_name = &import.module;
            // Check the module exists in the AST
            if !self
                .ast
                .modules
                .iter()
                .any(|m| &m.name == mod_name)
            {
                return Err(ResolverError::UnknownModule(mod_name.clone()));
            }
            // bring imported names into module scope
            for alias in &import.aliases {
                self.module_scope
                    .insert(alias.local.clone(), BindingKind::Module);
            }
            for func_name in &import.functions {
                // We can't know arity without the referenced module's AST here;
                // use arity 0 as a placeholder — full type-checking handles this
                self.module_scope
                    .insert(func_name.clone(), BindingKind::Function(0));
            }
        }

        // Resolve each item
        for item in &module.body {
            self.resolve_expr(item)?;
        }

        Ok(())
    }

    fn resolve_expr(&mut self, expr: &Expr) -> Result<(), ResolverError> {
        match expr {
            Expr::Integer(_)
            | Expr::Float(_)
            | Expr::Bool(_)
            | Expr::Nil
            | Expr::StringLit(_)
            | Expr::Atom(_)
            | Expr::CharLiteral(_) => Ok(()),

            Expr::Var(name) => {
                if self.lookup(name).is_none() && !self.is_builtin(name) {
                    Err(ResolverError::UnboundVariable(name.clone()))
                } else {
                    Ok(())
                }
            }

            Expr::Tuple(elems) | Expr::List(elems) => {
                for e in elems {
                    self.resolve_expr(e)?;
                }
                Ok(())
            }

            Expr::Map(pairs) => {
                for (k, v) in pairs {
                    self.resolve_expr(k)?;
                    self.resolve_expr(v)?;
                }
                Ok(())
            }

            Expr::BinaryOp { left, right, .. } => {
                self.resolve_expr(left)?;
                self.resolve_expr(right)?;
                Ok(())
            }

            Expr::UnaryOp { operand, .. } => self.resolve_expr(operand),

            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.resolve_expr(condition)?;
                self.resolve_expr(then_branch)?;
                if let Some(eb) = else_branch {
                    self.resolve_expr(eb)?;
                }
                Ok(())
            }

            Expr::Cond { clauses } => {
                for (cond, body) in clauses {
                    self.resolve_expr(cond)?;
                    self.resolve_expr(body)?;
                }
                Ok(())
            }

            Expr::Case { subject, clauses } => {
                self.resolve_expr(subject)?;
                for clause in clauses {
                    self.push_scope();
                    self.resolve_pattern(&clause.pattern)?;
                    if let Some(guard) = &clause.guard {
                        self.resolve_expr(guard)?;
                    }
                    self.resolve_expr(&clause.body)?;
                    self.pop_scope();
                }
                Ok(())
            }

            Expr::Receive { clauses, after } => {
                for clause in clauses {
                    self.push_scope();
                    self.resolve_pattern(&clause.pattern)?;
                    if let Some(guard) = &clause.guard {
                        self.resolve_expr(guard)?;
                    }
                    self.resolve_expr(&clause.body)?;
                    self.pop_scope();
                }
                if let Some((timeout, body)) = after {
                    self.resolve_expr(timeout)?;
                    self.resolve_expr(body)?;
                }
                Ok(())
            }

            Expr::Fn {
                params,
                body,
                guards,
            } => {
                self.push_scope();
                for p in params {
                    self.resolve_pattern(p)?;
                }
                if let Some(gs) = guards {
                    for g in gs {
                        self.resolve_expr(g)?;
                    }
                }
                self.resolve_expr(body)?;
                self.pop_scope();
                Ok(())
            }

            Expr::Call { callee, args } => {
                self.resolve_expr(callee)?;
                for a in args {
                    self.resolve_expr(a)?;
                }
                Ok(())
            }

            Expr::Pipe { left, right } => {
                self.resolve_expr(left)?;
                self.resolve_expr(right)?;
                Ok(())
            }

            Expr::Let { pattern, value, body } => {
                self.resolve_expr(value)?;
                self.push_scope();
                self.resolve_pattern(pattern)?;
                self.resolve_expr(body)?;
                self.pop_scope();
                Ok(())
            }

            Expr::Block(exprs) => {
                for e in exprs {
                    self.resolve_expr(e)?;
                }
                Ok(())
            }

            Expr::Def(d) => {
                self.push_scope();
                for arg in &d.args {
                    self.resolve_pattern(arg)?;
                }
                if let Some(guards) = &d.guards {
                    for g in guards {
                        self.resolve_expr(g)?;
                    }
                }
                self.resolve_expr(&d.body)?;
                self.pop_scope();
                Ok(())
            }

            Expr::DefMacro(dm) => {
                self.push_scope();
                for arg in &dm.args {
                    self.define(arg.clone(), BindingKind::Variable);
                }
                self.resolve_expr(&dm.body)?;
                self.pop_scope();
                Ok(())
            }

            Expr::Defstruct(ds) => {
                for field in &ds.fields {
                    if let Some(default) = &field.default {
                        self.resolve_expr(default)?;
                    }
                }
                Ok(())
            }

            Expr::Defprotocol(_) => Ok(()),

            Expr::Defimpl(di) => {
                // Validate the protocol exists
                let proto_name = &di.protocol;
                if self.lookup(proto_name) != Some(&BindingKind::Protocol) {
                    // Check in other modules
                    let found = self.ast.modules.iter().any(|m| {
                        m.body.iter().any(|e| {
                            matches!(e, Expr::Defprotocol(dp) if &dp.name == proto_name)
                        })
                    });
                    if !found {
                        return Err(ResolverError::UnknownProtocol(proto_name.clone()));
                    }
                }

                // Validate methods match protocol signature
                let protocol_methods = self.get_protocol_methods(proto_name);
                for method in &di.methods {
                    // Check method exists in protocol
                    let proto_method = protocol_methods
                        .iter()
                        .find(|(name, _)| name == &method.name);
                    if let Some((_, expected_arity)) = proto_method {
                        let actual_arity = method.args.len();
                        if actual_arity != *expected_arity {
                            return Err(ResolverError::ArityMismatch {
                                protocol: di.protocol.clone(),
                                target: di.target.clone(),
                                method: method.name.clone(),
                                expected: *expected_arity,
                                got: actual_arity,
                            });
                        }
                    }

                    // Resolve method body
                    self.push_scope();
                    for arg in &method.args {
                        self.resolve_pattern(arg)?;
                    }
                    self.resolve_expr(&method.body)?;
                    self.pop_scope();
                }
                Ok(())
            }

            Expr::Raise(e) => self.resolve_expr(e),

            Expr::Try {
                body,
                rescue_clauses,
                after,
            } => {
                self.resolve_expr(body)?;
                for clause in rescue_clauses {
                    self.push_scope();
                    if let Some(var) = &clause.binding {
                        self.define(var.clone(), BindingKind::Variable);
                    }
                    self.resolve_expr(&clause.body)?;
                    self.pop_scope();
                }
                if let Some(a) = after {
                    self.resolve_expr(a)?;
                }
                Ok(())
            }

            Expr::With {
                clauses,
                body,
                else_clauses,
            } => {
                self.push_scope();
                for clause in clauses {
                    self.resolve_expr(&clause.value)?;
                    self.resolve_pattern(&clause.pattern)?;
                }
                self.resolve_expr(body)?;
                self.pop_scope();
                for ec in else_clauses {
                    self.push_scope();
                    self.resolve_pattern(&ec.pattern)?;
                    self.resolve_expr(&ec.body)?;
                    self.pop_scope();
                }
                Ok(())
            }

            Expr::Comprehension {
                kind: _,
                body,
                generators,
                filters,
            } => {
                self.push_scope();
                for gen in generators {
                    self.resolve_expr(&gen.source)?;
                    self.resolve_pattern(&gen.pattern)?;
                }
                for f in filters {
                    self.resolve_expr(f)?;
                }
                self.resolve_expr(body)?;
                self.pop_scope();
                Ok(())
            }

            Expr::StructLit { name, fields } => {
                // Verify struct exists
                if self.lookup(name).is_none() {
                    let found = self.ast.modules.iter().any(|m| {
                        m.body.iter().any(|e| {
                            matches!(e, Expr::Defstruct(ds) if &ds.name == name)
                        })
                    });
                    if !found {
                        return Err(ResolverError::UnknownStruct(name.clone()));
                    }
                }
                for (_, v) in fields {
                    self.resolve_expr(v)?;
                }
                Ok(())
            }

            Expr::StructUpdate { base, fields } => {
                self.resolve_expr(base)?;
                for (_, v) in fields {
                    self.resolve_expr(v)?;
                }
                Ok(())
            }

            Expr::FieldAccess { object, .. } => self.resolve_expr(object),

            Expr::Index { object, index } => {
                self.resolve_expr(object)?;
                self.resolve_expr(index)?;
                Ok(())
            }

            Expr::Assign { target, value } => {
                self.resolve_expr(value)?;
                self.resolve_pattern(target)?;
                Ok(())
            }

            Expr::Send { receiver, message } => {
                self.resolve_expr(receiver)?;
                self.resolve_expr(message)?;
                Ok(())
            }

            Expr::Spawn(e) => self.resolve_expr(e),

            Expr::SelfRef => Ok(()),

            Expr::ModuleAccess { module, .. } => {
                if self.lookup(module).is_none() {
                    let known = self.ast.modules.iter().any(|m| &m.name == module);
                    if !known {
                        return Err(ResolverError::UnknownModule(module.clone()));
                    }
                }
                Ok(())
            }

            Expr::StringInterpolation(parts) => {
                for part in parts {
                    self.resolve_expr(part)?;
                }
                Ok(())
            }

            Expr::Range { start, end, step } => {
                self.resolve_expr(start)?;
                self.resolve_expr(end)?;
                if let Some(s) = step {
                    self.resolve_expr(s)?;
                }
                Ok(())
            }

            Expr::Slice {
                object,
                start,
                end,
            } => {
                self.resolve_expr(object)?;
                if let Some(s) = start {
                    self.resolve_expr(s)?;
                }
                if let Some(e) = end {
                    self.resolve_expr(e)?;
                }
                Ok(())
            }

            Expr::Return(e) => {
                if let Some(inner) = e {
                    self.resolve_expr(inner)?;
                }
                Ok(())
            }

            Expr::Throw(e) => self.resolve_expr(e),

            Expr::Await(e) => self.resolve_expr(e),

            Expr::Async(e) => self.resolve_expr(e),

            Expr::TypeAnnotation { expr, .. } => self.resolve_expr(expr),

            Expr::MacroCall { args, .. } => {
                for a in args {
                    self.resolve_expr(a)?;
                }
                Ok(())
            }

            Expr::UseDirective(_) => Ok(()),
            Expr::AliasDirective(_) => Ok(()),
            Expr::ImportDirective(_) => Ok(()),
            Expr::RequireDirective(_) => Ok(()),

            Expr::MultiClauseDef(mc) => {
                for clause in &mc.clauses {
                    self.push_scope();
                    for arg in &clause.args {
                        self.resolve_pattern(arg)?;
                    }
                    if let Some(guards) = &clause.guards {
                        for g in guards {
                            self.resolve_expr(g)?;
                        }
                    }
                    self.resolve_expr(&clause.body)?;
                    self.pop_scope();
                }
                Ok(())
            }

            Expr::CaptureFn { module, name, arity } => {
                let _ = (module, name, arity); // validated at type-check time
                Ok(())
            }

            Expr::BitString(segs) => {
                for seg in segs {
                    self.resolve_expr(&seg.value)?;
                    if let Some(sz) = &seg.size {
                        self.resolve_expr(sz)?;
                    }
                }
                Ok(())
            }
        }
    }

    fn resolve_pattern(&mut self, pattern: &Pattern) -> Result<(), ResolverError> {
        match pattern {
            Pattern::Wildcard => Ok(()),
            Pattern::Literal(_) => Ok(()),
            Pattern::Var(name) => {
                self.define(name.clone(), BindingKind::Variable);
                Ok(())
            }
            Pattern::Tuple(pats) | Pattern::List(pats) => {
                for p in pats {
                    self.resolve_pattern(p)?;
                }
                Ok(())
            }
            Pattern::Cons(head, tail) => {
                self.resolve_pattern(head)?;
                self.resolve_pattern(tail)?;
                Ok(())
            }
            Pattern::Map(pairs) => {
                for (_, v) in pairs {
                    self.resolve_pattern(v)?;
                }
                Ok(())
            }
            Pattern::Struct { fields, .. } => {
                for (_, v) in fields {
                    self.resolve_pattern(v)?;
                }
                Ok(())
            }
            Pattern::Or(pats) => {
                for p in pats {
                    self.resolve_pattern(p)?;
                }
                Ok(())
            }
            Pattern::Pin(name) => {
                if self.lookup(name).is_none() {
                    Err(ResolverError::UnboundVariable(name.clone()))
                } else {
                    Ok(())
                }
            }
            Pattern::BitString(segs) => {
                for seg in segs {
                    match &seg.pattern {
                        crate::parser::BitStringPatternKind::Var(v) => {
                            self.define(v.clone(), BindingKind::Variable);
                        }
                        crate::parser::BitStringPatternKind::Literal(_) => {}
                        crate::parser::BitStringPatternKind::Wildcard => {}
                    }
                    if let Some(sz) = &seg.size {
                        self.resolve_expr(sz)?;
                    }
                }
                Ok(())
            }
        }
    }

    fn is_builtin(&self, name: &str) -> bool {
        matches!(
            name,
            "true"
                | "false"
                | "nil"
                | "self"
                | "IO"
                | "String"
                | "List"
                | "Enum"
                | "Map"
                | "Tuple"
                | "Integer"
                | "Float"
                | "Atom"
                | "Process"
                | "System"
                | "File"
                | "Path"
                | "Keyword"
                | "Regex"
                | "DateTime"
                | "Date"
                | "Time"
                | "Base"
                | "Jason"
                | "Logger"
                | "Agent"
                | "Task"
                | "GenServer"
                | "Supervisor"
                | "Application"
                | "Mix"
                | "ExUnit"
                | "inspect"
                | "raise"
                | "throw"
                | "exit"
                | "send"
                | "spawn"
                | "receive"
                | "__MODULE__"
                | "__DIR__"
                | "__ENV__"
                | "__CALLER__"
                | "is_integer"
                | "is_float"
                | "is_binary"
                | "is_atom"
                | "is_list"
                | "is_tuple"
                | "is_map"
                | "is_nil"
                | "is_boolean"
                | "is_function"
                | "is_pid"
                | "is_reference"
                | "is_port"
                | "is_number"
                | "hd"
                | "tl"
                | "length"
                | "abs"
                | "rem"
                | "div"
                | "max"
                | "min"
                | "elem"
                | "put_elem"
                | "tuple_size"
                | "map_size"
                | "byte_size"
                | "bit_size"
                | "node"
                | "self"
                | "__using__"
                | "use"
        )
    }

    fn get_protocol_methods(&self, protocol_name: &str) -> Vec<(String, usize)> {
        for module in &self.ast.modules {
            for item in &module.body {
                if let Expr::Defprotocol(dp) = item {
                    if dp.name == protocol_name {
                        return dp
                            .methods
                            .iter()
                            .map(|m| (m.name.clone(), m.args.len()))
                            .collect();
                    }
                }
            }
        }
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn resolve(src: &str) -> Result<(), ResolverError> {
        let ast = parse(src).expect("parse failed");
        resolve_ast(&ast)
    }

    #[test]
    fn test_unbound_variable() {
        let err = resolve("defmodule M do\n  def f(), do: x\nend").unwrap_err();
        assert!(matches!(err, ResolverError::UnboundVariable(v) if v == "x"));
    }

    #[test]
    fn test_duplicate_module() {
        let err = resolve("defmodule M do end\ndefmodule M do end").unwrap_err();
        assert!(matches!(err, ResolverError::DuplicateModule(m) if m == "M"));
    }

    #[test]
    fn test_cyclic_dependency() {
        let err = resolve(
            "defmodule A do\n  import B\nend\ndefmodule B do\n  import A\nend",
        )
        .unwrap_err();
        assert!(matches!(err, ResolverError::CyclicDependency(_)));
    }

    #[test]
    fn test_unknown_module_import() {
        let err = resolve("defmodule A do\n  import NonExistent\nend").unwrap_err();
        assert!(matches!(err, ResolverError::UnknownModule(m) if m == "NonExistent"));
    }

    #[test]
    fn test_unknown_protocol() {
        let err = resolve(
            "defmodule A do\n  defimpl NoProto, for: Integer do\n    def size(x), do: x\n  end\nend",
        )
        .unwrap_err();
        assert!(matches!(err, ResolverError::UnknownProtocol(p) if p == "NoProto"));
    }

    #[test]
    fn test_protocol_arity_mismatch() {
        let src = "
defmodule A do
  defprotocol Size do
    def size(x)
  end

  defimpl Size, for: Tuple do
    def size(x, y), do: x + y
  end
end
";
        let err = resolve(src).unwrap_err();
        let idProtocolImpl = ResolverError::ArityMismatch {
            protocol: "Size".to_string(),
            target: "Tuple".to_string(),
            method: "size".to_string(),
            expected: 1,
            got: 2,
        };
        assert_eq!(err, idProtocolImpl);
        assert_eq!(
            error.to_string(),
            "[E1010] invalid defimpl for protocol 'Size' target 'Tuple': size/2 has arity mismatch (expected 1)"
        );
    }
}
