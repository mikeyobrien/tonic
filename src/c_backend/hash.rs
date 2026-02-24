use crate::ir::{IrOp, IrPattern};
use std::collections::BTreeSet;

use super::error::CBackendError;

pub(super) fn hash_text_i64(value: &str) -> i64 {
    hash_bytes_i64(value.as_bytes())
}

pub(super) fn hash_pattern_i64(pattern: &IrPattern) -> Result<i64, CBackendError> {
    let serialized = serde_json::to_string(pattern).map_err(|error| {
        CBackendError::new(format!(
            "c backend failed to serialize pattern hash input: {error}"
        ))
    })?;
    Ok(hash_bytes_i64(serialized.as_bytes()))
}

pub(super) fn hash_ir_op_i64(op: &IrOp) -> Result<i64, CBackendError> {
    let serialized = serde_json::to_string(op).map_err(|error| {
        CBackendError::new(format!(
            "c backend failed to serialize ir op hash input: {error}"
        ))
    })?;
    Ok(hash_bytes_i64(serialized.as_bytes()))
}

pub(super) fn hash_closure_descriptor_i64(
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

pub(super) fn closure_capture_names(params: &[String], ops: &[IrOp]) -> Vec<String> {
    let mut captures = BTreeSet::new();
    let param_names = params.iter().cloned().collect::<BTreeSet<_>>();
    collect_capture_names_from_ops(ops, &param_names, &mut captures);
    captures.into_iter().collect()
}

fn collect_capture_names_from_ops(
    ops: &[IrOp],
    params: &BTreeSet<String>,
    captures: &mut BTreeSet<String>,
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
