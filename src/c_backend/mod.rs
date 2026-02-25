mod decls;
mod dispatcher;
mod error;
mod funcs;
mod groups;
mod hash;
mod ops;
mod runtime_patterns;
mod stubs;
mod terminator;

pub(crate) use error::CBackendError;

use crate::llvm_backend::mangle_function_name;
use crate::mir::MirProgram;
use std::collections::BTreeMap;

use decls::{emit_forward_declarations, emit_main_entrypoint};
use dispatcher::emit_dispatcher;
use funcs::emit_function;
use groups::{group_functions, group_requires_dispatcher};
use stubs::{emit_header, emit_runtime_stubs};

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
    emit_forward_declarations(&groups, mir, &clause_symbols, &callable_symbols, &mut out);
    emit_runtime_stubs(mir, &mut out)?;

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
