use crate::ir::{CmpKind, IrCallTarget, IrOp, IrPattern};
use crate::llvm_backend::{instruction_name, mangle_function_name};
use crate::mir::{MirBinaryKind, MirBlock, MirFunction, MirInstruction, MirProgram, MirTerminator};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CBackendError {
    message: String,
}

impl CBackendError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub(crate) fn unsupported_instruction(
        function: &str,
        instruction: &MirInstruction,
        offset: usize,
    ) -> Self {
        let op = instruction_name(instruction);
        Self::new(format!(
            "c backend unsupported instruction {op} in function {function} at offset {offset}"
        ))
    }
}

impl fmt::Display for CBackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CBackendError {}

/// Lower a MIR program to a self-contained C source file.
///
/// The generated file:
/// - Includes required headers
/// - Declares `tn_runtime_*` functions as weak stubs (abort on call)
/// - Emits user function implementations
/// - Emits a `main()` that calls `Demo.run()` and prints the integer result
pub(crate) fn lower_mir_to_c(mir: &MirProgram) -> Result<String, CBackendError> {
    let groups = group_functions(mir);
    let mut callable_symbols = BTreeMap::<(String, usize), String>::new();
    let mut clause_symbols = BTreeMap::<usize, String>::new();

    for group in &groups {
        let dispatcher_symbol = mangle_function_name(&group.name, group.arity);
        callable_symbols.insert((group.name.clone(), group.arity), dispatcher_symbol.clone());

        let use_dispatcher = group_requires_dispatcher(group, mir);
        if !use_dispatcher {
            clause_symbols.insert(group.clause_indices[0], dispatcher_symbol);
            continue;
        }

        for (clause_index, function_index) in group.clause_indices.iter().copied().enumerate() {
            clause_symbols.insert(
                function_index,
                format!("{dispatcher_symbol}__clause{clause_index}"),
            );
        }
    }

    let mut out = String::new();

    emit_header(&mut out);
    emit_runtime_stubs(&mut out);
    emit_forward_declarations(&groups, mir, &clause_symbols, &callable_symbols, &mut out);

    for group in &groups {
        let use_dispatcher = group_requires_dispatcher(group, mir);
        if !use_dispatcher {
            let function_index = group.clause_indices[0];
            let function = &mir.functions[function_index];
            let symbol = clause_symbols
                .get(&function_index)
                .expect("clause symbol should exist for single-clause function");
            emit_function(function, symbol, &callable_symbols, &mut out)?;
            continue;
        }

        for function_index in &group.clause_indices {
            let function = &mir.functions[*function_index];
            let symbol = clause_symbols
                .get(function_index)
                .expect("clause symbol should exist for multi-clause function");
            emit_function(function, symbol, &callable_symbols, &mut out)?;
        }

        emit_dispatcher(group, mir, &clause_symbols, &callable_symbols, &mut out)?;
    }

    emit_main_entrypoint(&callable_symbols, &mut out);

    Ok(out)
}

// ---------------------------------------------------------------------------
// Header + runtime stubs
// ---------------------------------------------------------------------------

fn emit_header(out: &mut String) {
    out.push_str("/* tonic c backend - generated file */\n");
    out.push_str("#include <stdio.h>\n");
    out.push_str("#include <stdlib.h>\n");
    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <inttypes.h>\n");
    out.push_str("#include <string.h>\n");
    out.push_str("#include <stdarg.h>\n");
    out.push('\n');
    out.push_str("typedef int64_t TnVal;\n");
    out.push('\n');
}

/// Emit weak stub definitions for every `tn_runtime_*` function declared in
/// the LLVM backend.  Stubs abort with a descriptive message so programs that
/// only use integer arithmetic compile and run correctly, while programs that
/// depend on unimplemented runtime operations fail at runtime with a clear
/// diagnostic rather than a linker error.
fn emit_runtime_stubs(out: &mut String) {
    out.push_str("/* runtime stubs - operations not yet natively implemented */\n");
    out.push_str("static TnVal tn_stub_abort(const char *name) {\n");
    out.push_str("  fprintf(stderr, \"error: native runtime not available for '%s'\\n\", name);\n");
    out.push_str("  exit(1);\n");
    out.push_str("}\n\n");

    // Unary / error stubs
    for name in &[
        "tn_runtime_error_no_matching_clause",
        "tn_runtime_error_bad_match",
        "tn_runtime_error_arity_mismatch",
    ] {
        out.push_str(&format!(
            "static TnVal {name}(void) {{ return tn_stub_abort(\"{name}\"); }}\n"
        ));
    }
    out.push('\n');

    // Single-arg stubs
    for name in &[
        "tn_runtime_make_ok",
        "tn_runtime_make_err",
        "tn_runtime_question",
        "tn_runtime_raise",
        "tn_runtime_try",
        "tn_runtime_const_atom",
        "tn_runtime_load_binding",
        "tn_runtime_protocol_dispatch",
    ] {
        out.push_str(&format!(
            "static TnVal {name}(TnVal _a) {{ return tn_stub_abort(\"{name}\"); }}\n"
        ));
    }
    out.push('\n');

    // Two-arg stubs
    for name in &[
        "tn_runtime_match_operator",
        "tn_runtime_make_tuple",
        "tn_runtime_make_map",
        "tn_runtime_make_keyword",
        "tn_runtime_concat",
        "tn_runtime_in",
        "tn_runtime_list_concat",
        "tn_runtime_list_subtract",
        "tn_runtime_range",
    ] {
        out.push_str(&format!(
            "static TnVal {name}(TnVal _a, TnVal _b) {{ return tn_stub_abort(\"{name}\"); }}\n"
        ));
    }
    out.push('\n');

    // Three-arg stubs
    for name in &[
        "tn_runtime_make_closure",
        "tn_runtime_map_put",
        "tn_runtime_map_update",
        "tn_runtime_map_access",
        "tn_runtime_keyword_append",
    ] {
        out.push_str(&format!(
            "static TnVal {name}(TnVal _a, TnVal _b, TnVal _c) {{ return tn_stub_abort(\"{name}\"); }}\n"
        ));
    }
    out.push('\n');

    // Zero-arg and special stubs
    out.push_str("static TnVal tn_runtime_map_empty(void) { return tn_stub_abort(\"tn_runtime_map_empty\"); }\n");
    out.push_str("static int tn_runtime_pattern_matches(TnVal _v, TnVal _p) { (void)tn_stub_abort(\"tn_runtime_pattern_matches\"); return 0; }\n");
    out.push('\n');

    // Variadic stubs: list construction, host calls, closure calls.
    // These use an explicit leading count argument followed by the elements.
    out.push_str("static TnVal tn_runtime_make_list_varargs(TnVal _count, ...) {\n");
    out.push_str("  return tn_stub_abort(\"tn_runtime_make_list\");\n");
    out.push_str("}\n");
    out.push_str("static TnVal tn_runtime_host_call_varargs(TnVal _count, ...) {\n");
    out.push_str("  return tn_stub_abort(\"tn_runtime_host_call\");\n");
    out.push_str("}\n");
    out.push_str(
        "static TnVal tn_runtime_call_closure_varargs(TnVal _closure, TnVal _count, ...) {\n",
    );
    out.push_str("  return tn_stub_abort(\"tn_runtime_call_closure\");\n");
    out.push_str("}\n");
    out.push('\n');
}

fn emit_forward_declarations(
    groups: &[FunctionGroup],
    mir: &MirProgram,
    clause_symbols: &BTreeMap<usize, String>,
    callable_symbols: &BTreeMap<(String, usize), String>,
    out: &mut String,
) {
    out.push_str("/* forward declarations */\n");
    for group in groups {
        let use_dispatcher = group_requires_dispatcher(group, mir);
        if use_dispatcher {
            for function_index in &group.clause_indices {
                let symbol = clause_symbols
                    .get(function_index)
                    .expect("clause symbol should exist");
                let function = &mir.functions[*function_index];
                let params = (0..function.params.len())
                    .map(|i| format!("TnVal _arg{i}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!("static TnVal {symbol}({params});\n"));
            }

            let dispatcher_symbol = callable_symbols
                .get(&(group.name.clone(), group.arity))
                .expect("dispatcher symbol should exist");
            let params = (0..group.arity)
                .map(|i| format!("TnVal _arg{i}"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("static TnVal {dispatcher_symbol}({params});\n"));
        } else {
            let function_index = group.clause_indices[0];
            let symbol = clause_symbols
                .get(&function_index)
                .expect("clause symbol should exist");
            let function = &mir.functions[function_index];
            let params = (0..function.params.len())
                .map(|i| format!("TnVal _arg{i}"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("static TnVal {symbol}({params});\n"));
        }
    }
    out.push('\n');
}

// ---------------------------------------------------------------------------
// Function emission
// ---------------------------------------------------------------------------

fn emit_function(
    function: &MirFunction,
    symbol: &str,
    callable_symbols: &BTreeMap<(String, usize), String>,
    out: &mut String,
) -> Result<(), CBackendError> {
    let params = function
        .params
        .iter()
        .enumerate()
        .map(|(i, _)| format!("TnVal _arg{i}"))
        .collect::<Vec<_>>()
        .join(", ");

    out.push_str(&format!("static TnVal {symbol}({params}) {{\n"));

    // Infer which register IDs correspond to block-arg (phi) slots.
    // These are registers used in a block but not produced by its instructions.
    let phi_ids = infer_block_phi_reg_ids(function);

    // Declare ALL registers as locals at the function top.  This must include
    // both instruction destinations and block-arg (phi) registers, because C
    // forbids jumping past variable declarations with `goto`.
    let mut all_regs = collect_all_dests(function);
    for ids in phi_ids.values() {
        for id in ids {
            if !all_regs.contains(id) {
                all_regs.push(*id);
            }
        }
    }
    all_regs.sort_unstable();

    if !all_regs.is_empty() {
        let decls = all_regs
            .iter()
            .map(|id| format!("v{id}"))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("  TnVal {decls};\n"));
    }

    for block in &function.blocks {
        let empty_ids = Vec::new();
        let phi_reg_ids = phi_ids.get(&block.id).unwrap_or(&empty_ids);
        out.push_str(&format!("  bb{}: ;\n", block.id));
        emit_c_instructions(function, block, callable_symbols, out)?;
        emit_c_terminator_with_phi(function, block, &phi_ids, callable_symbols, out)?;
        let _ = phi_reg_ids;
    }

    out.push_str("}\n\n");
    Ok(())
}

fn collect_all_dests(function: &MirFunction) -> Vec<u32> {
    let mut dests = Vec::new();
    for block in &function.blocks {
        for instruction in &block.instructions {
            if let Some(dest) = instruction_dest(instruction) {
                if !dests.contains(&dest) {
                    dests.push(dest);
                }
            }
        }
    }
    dests.sort_unstable();
    dests
}

fn instruction_dest(instruction: &MirInstruction) -> Option<u32> {
    match instruction {
        MirInstruction::ConstInt { dest, .. }
        | MirInstruction::ConstFloat { dest, .. }
        | MirInstruction::ConstBool { dest, .. }
        | MirInstruction::ConstNil { dest, .. }
        | MirInstruction::ConstString { dest, .. }
        | MirInstruction::ConstAtom { dest, .. }
        | MirInstruction::LoadVariable { dest, .. }
        | MirInstruction::Unary { dest, .. }
        | MirInstruction::Binary { dest, .. }
        | MirInstruction::Call { dest, .. }
        | MirInstruction::CallValue { dest, .. }
        | MirInstruction::MakeClosure { dest, .. }
        | MirInstruction::Question { dest, .. }
        | MirInstruction::MatchPattern { dest, .. } => Some(*dest),
        MirInstruction::Legacy { dest, .. } => *dest,
    }
}

fn instruction_operands(instruction: &MirInstruction) -> Vec<u32> {
    match instruction {
        MirInstruction::ConstInt { .. }
        | MirInstruction::ConstFloat { .. }
        | MirInstruction::ConstBool { .. }
        | MirInstruction::ConstNil { .. }
        | MirInstruction::ConstString { .. }
        | MirInstruction::ConstAtom { .. } => vec![],
        MirInstruction::LoadVariable { .. } => vec![],
        MirInstruction::Unary { input, .. } | MirInstruction::Question { input, .. } => {
            vec![*input]
        }
        MirInstruction::Binary { left, right, .. } => vec![*left, *right],
        MirInstruction::Call { args, .. } => args.clone(),
        MirInstruction::CallValue { callee, args, .. } => {
            let mut ops = vec![*callee];
            ops.extend(args);
            ops
        }
        MirInstruction::MakeClosure { .. } => vec![],
        MirInstruction::MatchPattern { input, .. } => vec![*input],
        MirInstruction::Legacy { .. } => vec![],
    }
}

fn terminator_operands(terminator: &MirTerminator) -> Vec<u32> {
    match terminator {
        MirTerminator::Return { value, .. } => vec![*value],
        MirTerminator::Jump { args, .. } => args.clone(),
        MirTerminator::ShortCircuit { condition, .. } => vec![*condition],
        MirTerminator::Match { scrutinee, .. } => vec![*scrutinee],
    }
}

/// Infer, for each block that has args, the register IDs that serve as phi
/// inputs (values that are used in the block but not defined there).
///
/// This mirrors the logic in `llvm_backend::codegen::infer_block_arg_value_ids`.
fn infer_block_phi_reg_ids(function: &MirFunction) -> BTreeMap<u32, Vec<u32>> {
    use std::collections::BTreeSet;

    let mut result = BTreeMap::new();

    for block in &function.blocks {
        if block.args.is_empty() {
            result.insert(block.id, Vec::new());
            continue;
        }

        let mut defined = BTreeSet::<u32>::new();
        let mut ordered_external = Vec::<u32>::new();

        for instruction in &block.instructions {
            for used in instruction_operands(instruction) {
                if !defined.contains(&used) && !ordered_external.contains(&used) {
                    ordered_external.push(used);
                }
            }
            if let Some(dest) = instruction_dest(instruction) {
                defined.insert(dest);
            }
        }

        for used in terminator_operands(&block.terminator) {
            if !defined.contains(&used) && !ordered_external.contains(&used) {
                ordered_external.push(used);
            }
        }

        result.insert(block.id, ordered_external);
    }

    result
}

fn emit_c_instructions(
    function: &MirFunction,
    block: &MirBlock,
    callable_symbols: &BTreeMap<(String, usize), String>,
    out: &mut String,
) -> Result<(), CBackendError> {
    for instruction in &block.instructions {
        match instruction {
            MirInstruction::ConstInt { dest, value, .. } => {
                out.push_str(&format!("  v{dest} = (TnVal){value}LL;\n"));
            }
            MirInstruction::ConstBool { dest, value, .. } => {
                out.push_str(&format!(
                    "  v{dest} = (TnVal){};\n",
                    if *value { 1 } else { 0 }
                ));
            }
            MirInstruction::ConstNil { dest, .. } => {
                out.push_str(&format!("  v{dest} = (TnVal)0;\n"));
            }
            MirInstruction::ConstAtom { dest, value, .. } => {
                let hash = hash_text_i64(value);
                out.push_str(&format!(
                    "  v{dest} = tn_runtime_const_atom((TnVal){hash}LL);\n"
                ));
            }
            MirInstruction::ConstString { dest, .. } | MirInstruction::ConstFloat { dest, .. } => {
                // Unsupported types: emit a runtime abort stub call with offset 0
                out.push_str(&format!(
                    "  v{dest} = tn_stub_abort(\"unsupported constant type\");\n"
                ));
            }
            MirInstruction::LoadVariable { dest, name, .. } => {
                if let Some(param_index) = function.params.iter().position(|p| p.name == *name) {
                    out.push_str(&format!("  v{dest} = _arg{param_index};\n"));
                } else {
                    let binding_hash = hash_text_i64(name);
                    out.push_str(&format!(
                        "  v{dest} = tn_runtime_load_binding((TnVal){binding_hash}LL);\n"
                    ));
                }
            }
            MirInstruction::Unary {
                dest,
                kind,
                input,
                offset,
                ..
            } => match kind {
                crate::mir::MirUnaryKind::Raise => {
                    out.push_str(&format!("  v{dest} = tn_runtime_raise(v{input});\n"));
                }
                _ => {
                    return Err(CBackendError::unsupported_instruction(
                        &function.name,
                        instruction,
                        *offset,
                    ));
                }
            },
            MirInstruction::Question { dest, input, .. } => {
                out.push_str(&format!("  v{dest} = tn_runtime_question(v{input});\n"));
            }
            MirInstruction::Binary {
                dest,
                kind,
                left,
                right,
                ..
            } => {
                emit_c_binary(*dest, kind, *left, *right, out);
            }
            MirInstruction::Call {
                dest,
                callee,
                args,
                offset,
                ..
            } => {
                emit_c_call(
                    *dest,
                    callee,
                    args,
                    callable_symbols,
                    &function.name,
                    *offset,
                    out,
                )?;
            }
            MirInstruction::CallValue {
                dest, callee, args, ..
            } => {
                // Variadic closure call via stub
                let all_args = std::iter::once(format!("v{callee}"))
                    .chain(std::iter::once(format!("(TnVal){}", args.len())))
                    .chain(args.iter().map(|a| format!("v{a}")))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(
                    "  v{dest} = tn_runtime_call_closure_varargs({all_args});\n"
                ));
            }
            MirInstruction::MakeClosure {
                dest, params, ops, ..
            } => {
                let capture_names = closure_capture_names(params, ops);
                let descriptor_hash = hash_closure_descriptor_i64(params, ops, &capture_names)?;
                out.push_str(&format!(
                    "  v{dest} = tn_runtime_make_closure((TnVal){descriptor_hash}LL, (TnVal){}, (TnVal){});\n",
                    params.len(),
                    capture_names.len()
                ));
            }
            MirInstruction::MatchPattern {
                dest,
                input,
                pattern,
                ..
            } => {
                let pattern_hash = hash_pattern_i64(pattern)?;
                out.push_str(&format!(
                    "  v{dest} = tn_runtime_match_operator(v{input}, (TnVal){pattern_hash}LL);\n"
                ));
            }
            MirInstruction::Legacy {
                dest,
                source,
                offset,
                ..
            } => {
                if let IrOp::Try { .. } = source {
                    let try_hash = hash_ir_op_i64(source)?;
                    let Some(dest) = dest else {
                        return Err(CBackendError::new(format!(
                            "c backend missing legacy destination in function {} at offset {offset}",
                            function.name
                        )));
                    };
                    out.push_str(&format!(
                        "  v{dest} = tn_runtime_try((TnVal){try_hash}LL);\n"
                    ));
                } else {
                    return Err(CBackendError::unsupported_instruction(
                        &function.name,
                        instruction,
                        *offset,
                    ));
                }
            }
        }
    }
    Ok(())
}

fn emit_c_binary(dest: u32, kind: &MirBinaryKind, left: u32, right: u32, out: &mut String) {
    match kind {
        MirBinaryKind::AddInt => out.push_str(&format!("  v{dest} = v{left} + v{right};\n")),
        MirBinaryKind::SubInt => out.push_str(&format!("  v{dest} = v{left} - v{right};\n")),
        MirBinaryKind::MulInt => out.push_str(&format!("  v{dest} = v{left} * v{right};\n")),
        MirBinaryKind::DivInt => out.push_str(&format!("  v{dest} = v{left} / v{right};\n")),
        MirBinaryKind::CmpIntEq => {
            out.push_str(&format!("  v{dest} = (v{left} == v{right}) ? 1 : 0;\n"))
        }
        MirBinaryKind::CmpIntNotEq => {
            out.push_str(&format!("  v{dest} = (v{left} != v{right}) ? 1 : 0;\n"))
        }
        MirBinaryKind::CmpIntLt => {
            out.push_str(&format!("  v{dest} = (v{left} < v{right}) ? 1 : 0;\n"))
        }
        MirBinaryKind::CmpIntLte => {
            out.push_str(&format!("  v{dest} = (v{left} <= v{right}) ? 1 : 0;\n"))
        }
        MirBinaryKind::CmpIntGt => {
            out.push_str(&format!("  v{dest} = (v{left} > v{right}) ? 1 : 0;\n"))
        }
        MirBinaryKind::CmpIntGte => {
            out.push_str(&format!("  v{dest} = (v{left} >= v{right}) ? 1 : 0;\n"))
        }
        MirBinaryKind::Concat => out.push_str(&format!(
            "  v{dest} = tn_runtime_concat(v{left}, v{right});\n"
        )),
        MirBinaryKind::In => {
            out.push_str(&format!("  v{dest} = tn_runtime_in(v{left}, v{right});\n"))
        }
        MirBinaryKind::PlusPlus => out.push_str(&format!(
            "  v{dest} = tn_runtime_list_concat(v{left}, v{right});\n"
        )),
        MirBinaryKind::MinusMinus => out.push_str(&format!(
            "  v{dest} = tn_runtime_list_subtract(v{left}, v{right});\n"
        )),
        MirBinaryKind::Range => out.push_str(&format!(
            "  v{dest} = tn_runtime_range(v{left}, v{right});\n"
        )),
    }
}

fn emit_c_call(
    dest: u32,
    callee: &IrCallTarget,
    args: &[u32],
    callable_symbols: &BTreeMap<(String, usize), String>,
    function_name: &str,
    offset: usize,
    out: &mut String,
) -> Result<(), CBackendError> {
    let rendered_args = args
        .iter()
        .map(|id| format!("v{id}"))
        .collect::<Vec<_>>()
        .join(", ");

    match callee {
        IrCallTarget::Builtin { name } => {
            emit_c_builtin_call(dest, name, args, function_name, offset, out)?;
        }
        IrCallTarget::Function { name } => {
            let key = (name.clone(), args.len());
            if let Some(symbol) = callable_symbols.get(&key) {
                out.push_str(&format!("  v{dest} = {symbol}({rendered_args});\n"));
                return Ok(());
            }

            if callable_symbols
                .keys()
                .any(|(candidate, _)| candidate == name)
            {
                out.push_str(&format!("  v{dest} = tn_runtime_error_arity_mismatch();\n"));
                return Ok(());
            }

            return Err(CBackendError::new(format!(
                "c backend unknown function call target {name} in function {function_name} at offset {offset}"
            )));
        }
    }
    Ok(())
}

fn emit_c_builtin_call(
    dest: u32,
    builtin: &str,
    args: &[u32],
    function_name: &str,
    offset: usize,
    out: &mut String,
) -> Result<(), CBackendError> {
    let rendered_args = args
        .iter()
        .map(|id| format!("v{id}"))
        .collect::<Vec<_>>()
        .join(", ");

    match builtin {
        "ok" => {
            if args.len() != 1 {
                return Err(CBackendError::new(format!(
                    "c backend builtin ok arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            out.push_str(&format!(
                "  v{dest} = tn_runtime_make_ok({rendered_args});\n"
            ));
        }
        "err" => {
            if args.len() != 1 {
                return Err(CBackendError::new(format!(
                    "c backend builtin err arity mismatch in function {function_name} at offset {offset}"
                )));
            }
            out.push_str(&format!(
                "  v{dest} = tn_runtime_make_err({rendered_args});\n"
            ));
        }
        "tuple" => {
            out.push_str(&format!(
                "  v{dest} = tn_runtime_make_tuple({rendered_args});\n"
            ));
        }
        "list" => {
            // Variadic: first arg is count, then elements
            let count = args.len();
            let count_then_args = std::iter::once(format!("(TnVal){count}"))
                .chain(args.iter().map(|id| format!("v{id}")))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "  v{dest} = tn_runtime_make_list_varargs({count_then_args});\n"
            ));
        }
        "map_empty" => {
            out.push_str(&format!("  v{dest} = tn_runtime_map_empty();\n"));
        }
        "map" => {
            out.push_str(&format!(
                "  v{dest} = tn_runtime_make_map({rendered_args});\n"
            ));
        }
        "map_put" => {
            out.push_str(&format!(
                "  v{dest} = tn_runtime_map_put({rendered_args});\n"
            ));
        }
        "map_update" => {
            out.push_str(&format!(
                "  v{dest} = tn_runtime_map_update({rendered_args});\n"
            ));
        }
        "map_access" => {
            out.push_str(&format!(
                "  v{dest} = tn_runtime_map_access({rendered_args});\n"
            ));
        }
        "keyword" => {
            out.push_str(&format!(
                "  v{dest} = tn_runtime_make_keyword({rendered_args});\n"
            ));
        }
        "keyword_append" => {
            out.push_str(&format!(
                "  v{dest} = tn_runtime_keyword_append({rendered_args});\n"
            ));
        }
        "host_call" => {
            let count = args.len();
            let count_then_args = std::iter::once(format!("(TnVal){count}"))
                .chain(args.iter().map(|id| format!("v{id}")))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "  v{dest} = tn_runtime_host_call_varargs({count_then_args});\n"
            ));
        }
        "protocol_dispatch" => {
            out.push_str(&format!(
                "  v{dest} = tn_runtime_protocol_dispatch({rendered_args});\n"
            ));
        }
        other => {
            return Err(CBackendError::new(format!(
                "c backend unsupported builtin call target {other} in function {function_name} at offset {offset}"
            )));
        }
    }
    Ok(())
}

fn emit_c_terminator_with_phi(
    function: &MirFunction,
    block: &MirBlock,
    phi_ids: &BTreeMap<u32, Vec<u32>>,
    callable_symbols: &BTreeMap<(String, usize), String>,
    out: &mut String,
) -> Result<(), CBackendError> {
    match &block.terminator {
        MirTerminator::Return { value, .. } => {
            out.push_str(&format!("  return v{value};\n"));
        }
        MirTerminator::Jump { target, args } => {
            // Assign to the phi registers of the target block before jumping.
            let empty = Vec::new();
            let target_phi_regs = phi_ids.get(target).unwrap_or(&empty);
            for (i, arg_val) in args.iter().enumerate() {
                if let Some(&phi_reg) = target_phi_regs.get(i) {
                    out.push_str(&format!("  v{phi_reg} = v{arg_val};\n"));
                }
            }
            out.push_str(&format!("  goto bb{target};\n"));
        }
        MirTerminator::ShortCircuit {
            op,
            condition,
            on_evaluate_rhs,
            on_short_circuit,
            ..
        } => {
            let cond_expr = format!("v{condition}");
            let (true_target, false_target) = match op {
                crate::mir::MirShortCircuitOp::AndAnd | crate::mir::MirShortCircuitOp::And => {
                    (on_evaluate_rhs, on_short_circuit)
                }
                crate::mir::MirShortCircuitOp::OrOr | crate::mir::MirShortCircuitOp::Or => {
                    (on_short_circuit, on_evaluate_rhs)
                }
            };
            out.push_str(&format!(
                "  if ({cond_expr} != 0) {{ goto bb{true_target}; }} else {{ goto bb{false_target}; }}\n"
            ));
        }
        MirTerminator::Match {
            scrutinee,
            arms,
            offset,
        } => {
            emit_c_match_terminator(
                function,
                block,
                *scrutinee,
                arms,
                *offset,
                callable_symbols,
                out,
            )?;
        }
    }
    Ok(())
}

fn emit_c_match_terminator(
    function: &MirFunction,
    block: &MirBlock,
    scrutinee: u32,
    arms: &[crate::mir::MirMatchArm],
    _offset: usize,
    callable_symbols: &BTreeMap<(String, usize), String>,
    out: &mut String,
) -> Result<(), CBackendError> {
    if arms.is_empty() {
        out.push_str("  tn_runtime_error_no_matching_clause();\n");
        out.push_str("  return 0; /* unreachable */\n");
        return Ok(());
    }

    for (arm_index, arm) in arms.iter().enumerate() {
        let cond = emit_c_pattern_condition(
            &function.name,
            &format!("v{scrutinee}"),
            &arm.pattern,
            &format!("match_b{}_arm{arm_index}", block.id),
            out,
        )?;

        let guard_cond = if let Some(guard_ops) = &arm.guard_ops {
            let guard_reg = format!("match_b{}_arm{arm_index}_guard", block.id);
            let guard_val = emit_c_guard_condition(
                &function.name,
                guard_ops,
                &function.params,
                &guard_reg,
                callable_symbols,
                out,
            )?;
            Some(guard_val)
        } else {
            None
        };

        let full_cond = match guard_cond {
            Some(gc) => format!("({cond}) && ({gc})"),
            None => cond,
        };

        if arm_index + 1 == arms.len() {
            out.push_str(&format!(
                "  if ({full_cond}) {{ goto bb{}; }} else {{ tn_runtime_error_no_matching_clause(); return 0; }}\n",
                arm.target
            ));
        } else {
            out.push_str(&format!(
                "  if ({full_cond}) {{ goto bb{}; }}\n",
                arm.target
            ));
        }
    }
    Ok(())
}

/// Returns a C expression (as a string) that is non-zero when `pattern`
/// matches `scrutinee_expr` (a C expression string, e.g. `"v5"` or `"_arg0"`).
fn emit_c_pattern_condition(
    _function_name: &str,
    scrutinee_expr: &str,
    pattern: &IrPattern,
    label: &str,
    out: &mut String,
) -> Result<String, CBackendError> {
    match pattern {
        IrPattern::Wildcard | IrPattern::Bind { .. } => Ok("1".to_string()),
        IrPattern::Integer { value } => Ok(format!("({scrutinee_expr} == {value}LL)")),
        IrPattern::Bool { value } => Ok(format!(
            "({scrutinee_expr} == {})",
            if *value { 1 } else { 0 }
        )),
        IrPattern::Nil => Ok(format!("({scrutinee_expr} == 0)")),
        _ => {
            let pattern_hash = hash_pattern_i64(pattern)?;
            let reg = format!("{label}_complex");
            out.push_str(&format!(
                "  int {reg} = tn_runtime_pattern_matches({scrutinee_expr}, (TnVal){pattern_hash}LL);\n"
            ));
            Ok(reg)
        }
    }
}

fn emit_c_guard_condition(
    function_name: &str,
    guard_ops: &[IrOp],
    params: &[crate::mir::MirTypedName],
    label: &str,
    callable_symbols: &BTreeMap<(String, usize), String>,
    out: &mut String,
) -> Result<String, CBackendError> {
    let mut stack: Vec<String> = Vec::new();

    for (index, op) in guard_ops.iter().enumerate() {
        match op {
            IrOp::LoadVariable { name, .. } => {
                if let Some(param_index) = params.iter().position(|p| &p.name == name) {
                    stack.push(format!("_arg{param_index}"));
                } else {
                    let binding_hash = hash_text_i64(name);
                    let reg = format!("{label}_load_{index}");
                    out.push_str(&format!(
                        "  TnVal {reg} = tn_runtime_load_binding((TnVal){binding_hash}LL);\n"
                    ));
                    stack.push(reg);
                }
            }
            IrOp::ConstInt { value, .. } => {
                let reg = format!("{label}_ci_{index}");
                out.push_str(&format!("  TnVal {reg} = (TnVal){value}LL;\n"));
                stack.push(reg);
            }
            IrOp::ConstBool { value, .. } => {
                let reg = format!("{label}_cb_{index}");
                out.push_str(&format!(
                    "  TnVal {reg} = (TnVal){};\n",
                    if *value { 1 } else { 0 }
                ));
                stack.push(reg);
            }
            IrOp::ConstNil { .. } => {
                let reg = format!("{label}_cn_{index}");
                out.push_str(&format!("  TnVal {reg} = 0;\n"));
                stack.push(reg);
            }
            IrOp::CmpInt { kind, .. } => {
                let right = stack.pop().ok_or_else(|| {
                    CBackendError::new(format!(
                        "c backend guard stack underflow in function {function_name}"
                    ))
                })?;
                let left = stack.pop().ok_or_else(|| {
                    CBackendError::new(format!(
                        "c backend guard stack underflow in function {function_name}"
                    ))
                })?;
                let op_str = match kind {
                    CmpKind::Eq => "==",
                    CmpKind::NotEq => "!=",
                    CmpKind::Lt => "<",
                    CmpKind::Lte => "<=",
                    CmpKind::Gt => ">",
                    CmpKind::Gte => ">=",
                };
                let reg = format!("{label}_cmp_{index}");
                out.push_str(&format!(
                    "  TnVal {reg} = ({left} {op_str} {right}) ? 1 : 0;\n"
                ));
                stack.push(reg);
            }
            IrOp::Bang { .. } => {
                // Convert any value to boolean: non-zero → 1, zero → 0
                let value = stack.pop().ok_or_else(|| {
                    CBackendError::new(format!(
                        "c backend guard stack underflow in function {function_name}"
                    ))
                })?;
                let reg = format!("{label}_bang_{index}");
                out.push_str(&format!("  TnVal {reg} = ({value} != 0) ? 1 : 0;\n"));
                stack.push(reg);
            }
            IrOp::Not { .. } => {
                // Logical NOT: zero → 1, non-zero → 0
                let value = stack.pop().ok_or_else(|| {
                    CBackendError::new(format!(
                        "c backend guard stack underflow in function {function_name}"
                    ))
                })?;
                let reg = format!("{label}_not_{index}");
                out.push_str(&format!("  TnVal {reg} = ({value} == 0) ? 1 : 0;\n"));
                stack.push(reg);
            }
            IrOp::Call {
                callee,
                argc,
                offset,
            } => {
                if stack.len() < *argc {
                    return Err(CBackendError::new(format!(
                        "c backend guard stack underflow in function {function_name}"
                    )));
                }
                let split_index = stack.len() - *argc;
                let call_args = stack.split_off(split_index);
                let rendered_args = call_args.join(", ");
                let reg = format!("{label}_call_{index}");

                match callee {
                    IrCallTarget::Function { name } => {
                        let target_key = (name.clone(), *argc);
                        if let Some(symbol) = callable_symbols.get(&target_key) {
                            out.push_str(&format!("  TnVal {reg} = {symbol}({rendered_args});\n"));
                        } else if callable_symbols
                            .keys()
                            .any(|(candidate, _)| candidate == name)
                        {
                            out.push_str(&format!(
                                "  TnVal {reg} = tn_runtime_error_arity_mismatch();\n"
                            ));
                        } else {
                            return Err(CBackendError::new(format!(
                                "c backend unknown guard call target {name} in function {function_name} at offset {offset}"
                            )));
                        }
                    }
                    IrCallTarget::Builtin { name } => {
                        let reg_clone = reg.clone();
                        out.push_str(&format!("  TnVal {reg_clone};\n"));
                        // emit via the builtin helper using index stubs
                        let args_u32: Vec<u32> = (0..*argc).map(|i| i as u32).collect();
                        // We inline a simplified version for guards
                        out.push_str(&format!(
                            "  {reg_clone} = tn_stub_abort(\"guard builtin {name}\");\n"
                        ));
                        let _ = args_u32;
                    }
                }
                stack.push(reg);
            }
            _ => {
                return Err(CBackendError::new(format!(
                    "c backend unsupported guard op in function {function_name}"
                )));
            }
        }
    }

    stack.pop().ok_or_else(|| {
        CBackendError::new(format!(
            "c backend empty guard stack in function {function_name}"
        ))
    })
}

// ---------------------------------------------------------------------------
// Dispatcher emission
// ---------------------------------------------------------------------------

fn emit_dispatcher(
    group: &FunctionGroup,
    mir: &MirProgram,
    clause_symbols: &BTreeMap<usize, String>,
    callable_symbols: &BTreeMap<(String, usize), String>,
    out: &mut String,
) -> Result<(), CBackendError> {
    let dispatcher_symbol = mangle_function_name(&group.name, group.arity);
    let params = (0..group.arity)
        .map(|i| format!("TnVal _arg{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    let call_args = (0..group.arity)
        .map(|i| format!("_arg{i}"))
        .collect::<Vec<_>>()
        .join(", ");

    out.push_str(&format!("static TnVal {dispatcher_symbol}({params}) {{\n"));

    for (clause_index, function_index) in group.clause_indices.iter().copied().enumerate() {
        let function = &mir.functions[function_index];
        let clause_symbol = clause_symbols
            .get(&function_index)
            .expect("clause symbol should exist");

        let mut condition_terms: Vec<String> = Vec::new();

        if let Some(patterns) = &function.param_patterns {
            for (param_index, pattern) in patterns.iter().enumerate() {
                let label = format!("disp{clause_index}_pat{param_index}");
                let cond = emit_c_pattern_condition(
                    &function.name,
                    &format!("_arg{param_index}"),
                    pattern,
                    &label,
                    out,
                )?;
                condition_terms.push(cond);
            }
        }

        if let Some(guard_ops) = &function.guard_ops {
            let guard_label = format!("disp{clause_index}_guard");
            let guard_cond = emit_c_guard_condition(
                &function.name,
                guard_ops,
                &function.params,
                &guard_label,
                callable_symbols,
                out,
            )?;
            condition_terms.push(guard_cond);
        }

        let full_cond = if condition_terms.is_empty() {
            "1".to_string()
        } else {
            condition_terms
                .iter()
                .map(|c| format!("({c})"))
                .collect::<Vec<_>>()
                .join(" && ")
        };

        if clause_index + 1 == group.clause_indices.len() {
            out.push_str(&format!(
                "  if ({full_cond}) {{ return {clause_symbol}({call_args}); }}\n"
            ));
            out.push_str("  return tn_runtime_error_no_matching_clause();\n");
        } else {
            out.push_str(&format!(
                "  if ({full_cond}) {{ return {clause_symbol}({call_args}); }}\n"
            ));
        }
    }

    out.push_str("}\n\n");
    Ok(())
}

// ---------------------------------------------------------------------------
// main() entrypoint
// ---------------------------------------------------------------------------

fn emit_main_entrypoint(callable_symbols: &BTreeMap<(String, usize), String>, out: &mut String) {
    let entry_symbol = callable_symbols
        .get(&("Demo.run".to_string(), 0))
        .cloned()
        .unwrap_or_else(|| "tn_runtime_error_no_matching_clause".to_string());

    out.push_str("int main(void) {\n");
    out.push_str(&format!("  TnVal result = {entry_symbol}();\n"));
    out.push_str("  printf(\"%\" PRId64 \"\\n\", (int64_t)result);\n");
    out.push_str("  return 0;\n");
    out.push_str("}\n");
}

// ---------------------------------------------------------------------------
// Function groups (same logic as LLVM backend)
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct FunctionGroup {
    name: String,
    arity: usize,
    clause_indices: Vec<usize>,
}

fn group_functions(mir: &MirProgram) -> Vec<FunctionGroup> {
    let mut groups = Vec::<FunctionGroup>::new();
    let mut positions = BTreeMap::<(String, usize), usize>::new();

    for (index, function) in mir.functions.iter().enumerate() {
        let key = (function.name.clone(), function.params.len());
        if let Some(position) = positions.get(&key) {
            groups[*position].clause_indices.push(index);
            continue;
        }
        positions.insert(key, groups.len());
        groups.push(FunctionGroup {
            name: function.name.clone(),
            arity: function.params.len(),
            clause_indices: vec![index],
        });
    }
    groups
}

fn group_requires_dispatcher(group: &FunctionGroup, mir: &MirProgram) -> bool {
    if group.clause_indices.len() > 1 {
        return true;
    }
    let function = &mir.functions[group.clause_indices[0]];
    function.param_patterns.is_some() || function.guard_ops.is_some()
}

// ---------------------------------------------------------------------------
// Hash helpers (same as LLVM backend)
// ---------------------------------------------------------------------------

fn hash_text_i64(value: &str) -> i64 {
    hash_bytes_i64(value.as_bytes())
}

fn hash_pattern_i64(pattern: &IrPattern) -> Result<i64, CBackendError> {
    let serialized = serde_json::to_string(pattern).map_err(|error| {
        CBackendError::new(format!(
            "c backend failed to serialize pattern hash input: {error}"
        ))
    })?;
    Ok(hash_bytes_i64(serialized.as_bytes()))
}

fn hash_ir_op_i64(op: &IrOp) -> Result<i64, CBackendError> {
    let serialized = serde_json::to_string(op).map_err(|error| {
        CBackendError::new(format!(
            "c backend failed to serialize ir op hash input: {error}"
        ))
    })?;
    Ok(hash_bytes_i64(serialized.as_bytes()))
}

fn hash_closure_descriptor_i64(
    params: &[String],
    ops: &[IrOp],
    capture_names: &[String],
) -> Result<i64, CBackendError> {
    let serialized = serde_json::to_string(&(params, ops, capture_names)).map_err(|error| {
        CBackendError::new(format!(
            "c backend failed to serialize closure descriptor hash input: {error}"
        ))
    })?;
    Ok(hash_bytes_i64(serialized.as_bytes()))
}

fn closure_capture_names(params: &[String], ops: &[IrOp]) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut captures = BTreeSet::new();
    let param_names = params.iter().cloned().collect::<BTreeSet<_>>();
    collect_capture_names_from_ops(ops, &param_names, &mut captures);
    captures.into_iter().collect()
}

fn collect_capture_names_from_ops(
    ops: &[IrOp],
    params: &std::collections::BTreeSet<String>,
    captures: &mut std::collections::BTreeSet<String>,
) {
    for op in ops {
        match op {
            IrOp::LoadVariable { name, .. } => {
                if !params.contains(name) {
                    captures.insert(name.clone());
                }
            }
            IrOp::AndAnd { right_ops, .. }
            | IrOp::OrOr { right_ops, .. }
            | IrOp::And { right_ops, .. }
            | IrOp::Or { right_ops, .. } => {
                collect_capture_names_from_ops(right_ops, params, captures);
            }
            IrOp::Case { branches, .. } => {
                for branch in branches {
                    if let Some(guard_ops) = &branch.guard_ops {
                        collect_capture_names_from_ops(guard_ops, params, captures);
                    }
                    collect_capture_names_from_ops(&branch.ops, params, captures);
                }
            }
            IrOp::Try {
                body_ops,
                rescue_branches,
                catch_branches,
                after_ops,
                ..
            } => {
                collect_capture_names_from_ops(body_ops, params, captures);
                for branch in rescue_branches {
                    if let Some(guard_ops) = &branch.guard_ops {
                        collect_capture_names_from_ops(guard_ops, params, captures);
                    }
                    collect_capture_names_from_ops(&branch.ops, params, captures);
                }
                for branch in catch_branches {
                    if let Some(guard_ops) = &branch.guard_ops {
                        collect_capture_names_from_ops(guard_ops, params, captures);
                    }
                    collect_capture_names_from_ops(&branch.ops, params, captures);
                }
                if let Some(after) = after_ops {
                    collect_capture_names_from_ops(after, params, captures);
                }
            }
            IrOp::For {
                generators,
                into_ops,
                body_ops,
                ..
            } => {
                for (_, gen_ops) in generators {
                    collect_capture_names_from_ops(gen_ops, params, captures);
                }
                if let Some(into) = into_ops {
                    collect_capture_names_from_ops(into, params, captures);
                }
                collect_capture_names_from_ops(body_ops, params, captures);
            }
            _ => {}
        }
    }
}

fn hash_bytes_i64(bytes: &[u8]) -> i64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    i64::from_ne_bytes(hash.to_ne_bytes())
}
