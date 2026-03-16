use std::collections::BTreeMap;

use crate::ir::IrOp;
use crate::mir::{MirInstruction, MirProgram};

use super::error::CBackendError;
use super::hash::{hash_ir_op_i64, hash_pattern_i64};

#[path = "stubs_try_ops.rs"]
mod ops;
use ops::emit_try_ops;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TrySpec {
    pub(super) hash: i64,
    pub(super) op: IrOp,
}

pub(super) fn emit_runtime_try_helpers(
    mir: &MirProgram,
    out: &mut String,
) -> Result<(), CBackendError> {
    let try_specs = collect_try_specs(mir)?;

    out.push_str("/* compiled try helpers */\n");
    for (index, try_spec) in try_specs.iter().enumerate() {
        emit_runtime_try_case(index, try_spec, out)?;
    }

    out.push_str("static TnVal tn_runtime_try(TnVal op_hash) {\n");
    if try_specs.is_empty() {
        out.push_str("  return tn_stub_abort(\"tn_runtime_try\");\n");
    } else {
        out.push_str("  switch (op_hash) {\n");
        for (index, try_spec) in try_specs.iter().enumerate() {
            out.push_str(&format!(
                "    case (TnVal){}LL: return tn_runtime_try_case_{index}();\n",
                try_spec.hash
            ));
        }
        out.push_str("    default:\n");
        out.push_str("      return tn_stub_abort(\"tn_runtime_try\");\n");
        out.push_str("  }\n");
    }
    out.push_str("}\n\n");

    Ok(())
}

fn collect_try_specs(mir: &MirProgram) -> Result<Vec<TrySpec>, CBackendError> {
    let mut by_hash = BTreeMap::<i64, IrOp>::new();

    for function in &mir.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                let MirInstruction::Legacy { source, .. } = instruction else {
                    continue;
                };

                if !matches!(source, IrOp::Try { .. }) {
                    continue;
                }

                let hash = hash_ir_op_i64(source)?;
                if let Some(existing) = by_hash.get(&hash) {
                    if existing != source {
                        return Err(CBackendError::new(format!(
                            "c backend try hash collision for hash {hash}"
                        )));
                    }
                } else {
                    by_hash.insert(hash, source.clone());
                }
            }
        }
    }

    Ok(by_hash
        .into_iter()
        .map(|(hash, op)| TrySpec { hash, op })
        .collect())
}

fn emit_runtime_try_case(
    index: usize,
    try_spec: &TrySpec,
    out: &mut String,
) -> Result<(), CBackendError> {
    let IrOp::Try {
        body_ops,
        rescue_branches,
        catch_branches,
        after_ops,
        ..
    } = &try_spec.op
    else {
        return Err(CBackendError::new(
            "c backend internal error: try case source was not IrOp::Try",
        ));
    };

    out.push_str(&format!(
        "static TnVal tn_runtime_try_case_{index}(void) {{\n"
    ));
    out.push_str("  TnBinding tn_try_bindings[TN_MAX_BINDINGS];\n");
    out.push_str("  size_t tn_try_bindings_len = 0;\n");
    out.push_str("  tn_binding_snapshot(tn_try_bindings, &tn_try_bindings_len);\n");
    out.push_str("  int tn_try_raised = 0;\n");
    out.push_str("  TnVal tn_try_error = tn_runtime_const_nil();\n");
    out.push_str("  TnVal tn_try_result = tn_runtime_const_nil();\n");

    emit_try_ops(
        body_ops,
        "tn_try_result",
        "tn_try_raised",
        "tn_try_error",
        &format!("tn_try_case_{index}_body"),
        "  ",
        out,
    )?;

    out.push_str("  if (tn_try_raised != 0) {\n");
    out.push_str("    int tn_try_handled = 0;\n");

    for (branch_index, branch) in rescue_branches.iter().enumerate() {
        let pattern_hash = hash_pattern_i64(&branch.pattern)?;
        out.push_str(&format!(
            "    if (tn_try_handled == 0 && tn_runtime_pattern_matches(tn_try_error, (TnVal){pattern_hash}LL)) {{\n"
        ));

        // Determine indent for the branch body — deeper when a guard wraps it.
        let body_indent = if branch.guard_ops.is_some() {
            "        "
        } else {
            "      "
        };

        if let Some(guard_ops) = &branch.guard_ops {
            let guard_label = format!("tn_try_case_{index}_rescue_{branch_index}_guard");
            out.push_str("      TnVal tn_guard_result = tn_runtime_const_nil();\n");
            out.push_str("      int tn_guard_raised = 0;\n");
            out.push_str("      TnVal tn_guard_error = tn_runtime_const_nil();\n");
            emit_try_ops(
                guard_ops,
                "tn_guard_result",
                "tn_guard_raised",
                "tn_guard_error",
                &guard_label,
                "      ",
                out,
            )?;
            out.push_str(
                "      if (tn_guard_raised == 0 && tn_runtime_is_truthy(tn_guard_result)) {\n",
            );
        }

        out.push_str(&format!("{body_indent}int tn_branch_raised = 0;\n"));
        out.push_str(&format!(
            "{body_indent}TnVal tn_branch_error = tn_runtime_const_nil();\n"
        ));
        out.push_str(&format!(
            "{body_indent}TnVal tn_branch_result = tn_runtime_const_nil();\n"
        ));
        emit_try_ops(
            &branch.ops,
            "tn_branch_result",
            "tn_branch_raised",
            "tn_branch_error",
            &format!("tn_try_case_{index}_rescue_{branch_index}"),
            body_indent,
            out,
        )?;
        out.push_str(&format!("{body_indent}if (tn_branch_raised != 0) {{\n"));
        out.push_str(&format!("{body_indent}  tn_try_raised = 1;\n"));
        out.push_str(&format!("{body_indent}  tn_try_error = tn_branch_error;\n"));
        out.push_str(&format!("{body_indent}}} else {{\n"));
        out.push_str(&format!("{body_indent}  tn_try_raised = 0;\n"));
        out.push_str(&format!(
            "{body_indent}  tn_try_result = tn_branch_result;\n"
        ));
        out.push_str(&format!("{body_indent}  tn_try_handled = 1;\n"));
        out.push_str(&format!("{body_indent}}}\n"));

        if branch.guard_ops.is_some() {
            out.push_str("      }\n");
        }

        out.push_str("    }\n");
    }

    for (branch_index, branch) in catch_branches.iter().enumerate() {
        let pattern_hash = hash_pattern_i64(&branch.pattern)?;
        out.push_str(&format!(
            "    if (tn_try_raised != 0 && tn_try_handled == 0 && tn_runtime_pattern_matches(tn_try_error, (TnVal){pattern_hash}LL)) {{\n"
        ));

        let body_indent = if branch.guard_ops.is_some() {
            "        "
        } else {
            "      "
        };

        if let Some(guard_ops) = &branch.guard_ops {
            let guard_label = format!("tn_try_case_{index}_catch_{branch_index}_guard");
            out.push_str("      TnVal tn_guard_result = tn_runtime_const_nil();\n");
            out.push_str("      int tn_guard_raised = 0;\n");
            out.push_str("      TnVal tn_guard_error = tn_runtime_const_nil();\n");
            emit_try_ops(
                guard_ops,
                "tn_guard_result",
                "tn_guard_raised",
                "tn_guard_error",
                &guard_label,
                "      ",
                out,
            )?;
            out.push_str(
                "      if (tn_guard_raised == 0 && tn_runtime_is_truthy(tn_guard_result)) {\n",
            );
        }

        out.push_str(&format!("{body_indent}int tn_branch_raised = 0;\n"));
        out.push_str(&format!(
            "{body_indent}TnVal tn_branch_error = tn_runtime_const_nil();\n"
        ));
        out.push_str(&format!(
            "{body_indent}TnVal tn_branch_result = tn_runtime_const_nil();\n"
        ));
        emit_try_ops(
            &branch.ops,
            "tn_branch_result",
            "tn_branch_raised",
            "tn_branch_error",
            &format!("tn_try_case_{index}_catch_{branch_index}"),
            body_indent,
            out,
        )?;
        out.push_str(&format!("{body_indent}if (tn_branch_raised != 0) {{\n"));
        out.push_str(&format!("{body_indent}  tn_try_raised = 1;\n"));
        out.push_str(&format!("{body_indent}  tn_try_error = tn_branch_error;\n"));
        out.push_str(&format!("{body_indent}}} else {{\n"));
        out.push_str(&format!("{body_indent}  tn_try_raised = 0;\n"));
        out.push_str(&format!(
            "{body_indent}  tn_try_result = tn_branch_result;\n"
        ));
        out.push_str(&format!("{body_indent}  tn_try_handled = 1;\n"));
        out.push_str(&format!("{body_indent}}}\n"));

        if branch.guard_ops.is_some() {
            out.push_str("      }\n");
        }

        out.push_str("    }\n");
    }

    out.push_str("  }\n");

    if let Some(after_ops) = after_ops {
        out.push_str("  int tn_after_raised = 0;\n");
        out.push_str("  TnVal tn_after_error = tn_runtime_const_nil();\n");
        out.push_str("  TnVal tn_after_result = tn_runtime_const_nil();\n");
        emit_try_ops(
            after_ops,
            "tn_after_result",
            "tn_after_raised",
            "tn_after_error",
            &format!("tn_try_case_{index}_after"),
            "  ",
            out,
        )?;
        out.push_str("  if (tn_after_raised != 0) {\n");
        out.push_str("    tn_try_raised = 1;\n");
        out.push_str("    tn_try_error = tn_after_error;\n");
        out.push_str("  }\n");
    }

    out.push_str("  if (tn_try_raised != 0) {\n");
    out.push_str("    TnObj *err_obj = tn_get_obj(tn_try_error);\n");
    out.push_str("    tn_binding_restore(tn_try_bindings, tn_try_bindings_len);\n");
    out.push_str("    if (err_obj != NULL && err_obj->kind == TN_OBJ_STRING) {\n");
    out.push_str("      return tn_runtime_fail(err_obj->as.text.text);\n");
    out.push_str("    }\n");
    out.push_str("    if (err_obj != NULL && err_obj->kind == TN_OBJ_ATOM) {\n");
    out.push_str("      return tn_runtime_fail(err_obj->as.text.text);\n");
    out.push_str("    }\n");
    out.push_str("    return tn_runtime_fail(\"exception raised\");\n");
    out.push_str("  }\n");

    out.push_str("  tn_binding_restore(tn_try_bindings, tn_try_bindings_len);\n");
    out.push_str("  return tn_try_result;\n");
    out.push_str("}\n\n");

    Ok(())
}
