use crate::ir::{IrOp, IrPattern};
use crate::mir::{MirInstruction, MirProgram, MirTerminator};
use std::collections::BTreeMap;

use super::error::CBackendError;
use super::hash::hash_pattern_i64;

#[path = "runtime_patterns_emit.rs"]
mod emit;
use emit::emit_pattern_case;

#[derive(Debug, Clone)]
struct PatternCase {
    hash: i64,
    symbol: String,
    pattern: IrPattern,
}

pub(super) fn emit_runtime_pattern_helpers(
    mir: &MirProgram,
    out: &mut String,
) -> Result<(), CBackendError> {
    let collected = collect_patterns(mir)?;
    let pattern_cases = collected
        .into_iter()
        .enumerate()
        .map(|(index, (hash, pattern))| PatternCase {
            hash,
            symbol: format!("tn_pattern_match_case_{index}"),
            pattern,
        })
        .collect::<Vec<_>>();

    out.push_str("/* pattern/binding runtime helpers */\n");
    out.push_str("#define TN_MAX_BINDINGS 256\n");
    out.push_str("typedef struct { TnVal key; TnVal value; } TnBinding;\n");
    out.push_str("static TnBinding tn_bindings[TN_MAX_BINDINGS];\n");
    out.push_str("static size_t tn_bindings_len = 0;\n");
    out.push('\n');
    out.push_str("static long tn_binding_find_index(TnVal key) {\n");
    out.push_str("  for (size_t i = 0; i < tn_bindings_len; i += 1) {\n");
    out.push_str("    if (tn_bindings[i].key == key) {\n");
    out.push_str("      return (long)i;\n");
    out.push_str("    }\n");
    out.push_str("  }\n");
    out.push('\n');
    out.push_str("  return -1;\n");
    out.push_str("}\n");
    out.push('\n');
    out.push_str("static int tn_binding_get(TnVal key, TnVal *value) {\n");
    out.push_str("  long index = tn_binding_find_index(key);\n");
    out.push_str("  if (index < 0) {\n");
    out.push_str("    return 0;\n");
    out.push_str("  }\n");
    out.push('\n');
    out.push_str("  *value = tn_bindings[index].value;\n");
    out.push_str("  return 1;\n");
    out.push_str("}\n");
    out.push('\n');
    out.push_str("static void tn_binding_set(TnVal key, TnVal value) {\n");
    out.push_str("  long index = tn_binding_find_index(key);\n");
    out.push_str("  if (index >= 0) {\n");
    out.push_str("    tn_bindings[index].value = value;\n");
    out.push_str("    return;\n");
    out.push_str("  }\n");
    out.push('\n');
    out.push_str("  if (tn_bindings_len >= TN_MAX_BINDINGS) {\n");
    out.push_str("    fprintf(stderr, \"error: native runtime binding capacity exceeded\\n\");\n");
    out.push_str("    exit(1);\n");
    out.push_str("  }\n");
    out.push('\n');
    out.push_str("  tn_bindings[tn_bindings_len].key = key;\n");
    out.push_str("  tn_bindings[tn_bindings_len].value = value;\n");
    out.push_str("  tn_bindings_len += 1;\n");
    out.push_str("}\n");
    out.push('\n');
    out.push_str("static void tn_binding_snapshot(TnBinding *snapshot, size_t *snapshot_len) {\n");
    out.push_str("  *snapshot_len = tn_bindings_len;\n");
    out.push_str("  if (tn_bindings_len > 0) {\n");
    out.push_str("    memcpy(snapshot, tn_bindings, tn_bindings_len * sizeof(TnBinding));\n");
    out.push_str("  }\n");
    out.push_str("}\n");
    out.push('\n');
    out.push_str(
        "static void tn_binding_restore(const TnBinding *snapshot, size_t snapshot_len) {\n",
    );
    out.push_str("  tn_bindings_len = snapshot_len;\n");
    out.push_str("  if (snapshot_len > 0) {\n");
    out.push_str("    memcpy(tn_bindings, snapshot, snapshot_len * sizeof(TnBinding));\n");
    out.push_str("  }\n");
    out.push_str("}\n");
    out.push('\n');
    out.push_str("static int tn_pattern_match_internal(TnVal value, TnVal pattern_hash);\n");
    for pattern_case in &pattern_cases {
        out.push_str(&format!(
            "static int {}(TnVal value);\n",
            pattern_case.symbol
        ));
    }
    out.push('\n');

    out.push_str("static TnVal tn_runtime_load_binding(TnVal key) {\n");
    out.push_str("  TnVal value = 0;\n");
    out.push_str("  if (tn_binding_get(key, &value)) {\n");
    out.push_str("    return value;\n");
    out.push_str("  }\n");
    out.push('\n');
    out.push_str(
        "  return tn_runtime_failf(\"missing runtime binding for key %lld\", (long long)key);\n",
    );
    out.push_str("}\n");
    out.push('\n');
    out.push_str("static int tn_runtime_pattern_matches(TnVal value, TnVal pattern_hash) {\n");
    out.push_str("  TnBinding snapshot[TN_MAX_BINDINGS];\n");
    out.push_str("  size_t snapshot_len = 0;\n");
    out.push_str("  tn_binding_snapshot(snapshot, &snapshot_len);\n");
    out.push('\n');
    out.push_str("  if (tn_pattern_match_internal(value, pattern_hash)) {\n");
    out.push_str("    return 1;\n");
    out.push_str("  }\n");
    out.push('\n');
    out.push_str("  tn_binding_restore(snapshot, snapshot_len);\n");
    out.push_str("  return 0;\n");
    out.push_str("}\n");
    out.push('\n');
    out.push_str("static TnVal tn_runtime_match_operator(TnVal value, TnVal pattern_hash) {\n");
    out.push_str("  if (!tn_runtime_pattern_matches(value, pattern_hash)) {\n");
    out.push_str("    return tn_runtime_error_bad_match();\n");
    out.push_str("  }\n");
    out.push('\n');
    out.push_str("  return value;\n");
    out.push_str("}\n");
    out.push('\n');

    out.push_str("static int tn_pattern_match_internal(TnVal value, TnVal pattern_hash) {\n");
    out.push_str("  switch (pattern_hash) {\n");
    for pattern_case in &pattern_cases {
        out.push_str(&format!(
            "    case (TnVal){}LL: return {}(value);\n",
            pattern_case.hash, pattern_case.symbol
        ));
    }
    out.push_str("    default:\n");
    out.push_str("      (void)tn_stub_abort(\"tn_runtime_pattern_matches\");\n");
    out.push_str("      return 0;\n");
    out.push_str("  }\n");
    out.push_str("}\n");
    out.push('\n');

    for pattern_case in &pattern_cases {
        emit_pattern_case(pattern_case, out)?;
    }

    out.push('\n');
    Ok(())
}

fn collect_patterns(mir: &MirProgram) -> Result<BTreeMap<i64, IrPattern>, CBackendError> {
    let mut patterns = BTreeMap::<i64, IrPattern>::new();

    for function in &mir.functions {
        if let Some(param_patterns) = &function.param_patterns {
            for pattern in param_patterns {
                register_pattern(pattern, &mut patterns)?;
            }
        }

        for block in &function.blocks {
            for instruction in &block.instructions {
                match instruction {
                    MirInstruction::MatchPattern { pattern, .. } => {
                        register_pattern(pattern, &mut patterns)?;
                    }
                    MirInstruction::Legacy { source, .. } => {
                        register_patterns_from_op(source, &mut patterns)?;
                    }
                    _ => {}
                }
            }

            if let MirTerminator::Match { arms, .. } = &block.terminator {
                for arm in arms {
                    register_pattern(&arm.pattern, &mut patterns)?;
                }
            }
        }
    }

    Ok(patterns)
}

fn register_patterns_from_op(
    op: &IrOp,
    patterns: &mut BTreeMap<i64, IrPattern>,
) -> Result<(), CBackendError> {
    match op {
        IrOp::Match { pattern, .. } => {
            register_pattern(pattern, patterns)?;
        }
        IrOp::Case { branches, .. } => {
            for branch in branches {
                register_pattern(&branch.pattern, patterns)?;
                if let Some(guard_ops) = &branch.guard_ops {
                    for guard_op in guard_ops {
                        register_patterns_from_op(guard_op, patterns)?;
                    }
                }
                for branch_op in &branch.ops {
                    register_patterns_from_op(branch_op, patterns)?;
                }
            }
        }
        IrOp::Try {
            body_ops,
            rescue_branches,
            catch_branches,
            after_ops,
            ..
        } => {
            for body_op in body_ops {
                register_patterns_from_op(body_op, patterns)?;
            }
            for branch in rescue_branches {
                register_pattern(&branch.pattern, patterns)?;
                if let Some(guard_ops) = &branch.guard_ops {
                    for guard_op in guard_ops {
                        register_patterns_from_op(guard_op, patterns)?;
                    }
                }
                for branch_op in &branch.ops {
                    register_patterns_from_op(branch_op, patterns)?;
                }
            }
            for branch in catch_branches {
                register_pattern(&branch.pattern, patterns)?;
                if let Some(guard_ops) = &branch.guard_ops {
                    for guard_op in guard_ops {
                        register_patterns_from_op(guard_op, patterns)?;
                    }
                }
                for branch_op in &branch.ops {
                    register_patterns_from_op(branch_op, patterns)?;
                }
            }
            if let Some(after_ops) = after_ops {
                for after_op in after_ops {
                    register_patterns_from_op(after_op, patterns)?;
                }
            }
        }
        IrOp::For {
            generators,
            into_ops,
            reduce_ops,
            body_ops,
            ..
        } => {
            for generator in generators {
                register_pattern(&generator.pattern, patterns)?;
                for generator_op in &generator.source_ops {
                    register_patterns_from_op(generator_op, patterns)?;
                }
                if let Some(guard_ops) = &generator.guard_ops {
                    for guard_op in guard_ops {
                        register_patterns_from_op(guard_op, patterns)?;
                    }
                }
            }
            if let Some(into_ops) = into_ops {
                for into_op in into_ops {
                    register_patterns_from_op(into_op, patterns)?;
                }
            }
            if let Some(reduce_ops) = reduce_ops {
                for reduce_op in reduce_ops {
                    register_patterns_from_op(reduce_op, patterns)?;
                }
            }
            for body_op in body_ops {
                register_patterns_from_op(body_op, patterns)?;
            }
        }
        IrOp::AndAnd { right_ops, .. }
        | IrOp::OrOr { right_ops, .. }
        | IrOp::And { right_ops, .. }
        | IrOp::Or { right_ops, .. } => {
            for right_op in right_ops {
                register_patterns_from_op(right_op, patterns)?;
            }
        }
        _ => {}
    }

    Ok(())
}

fn register_pattern(
    pattern: &IrPattern,
    patterns: &mut BTreeMap<i64, IrPattern>,
) -> Result<(), CBackendError> {
    let hash = hash_pattern_i64(pattern)?;
    if let Some(existing) = patterns.get(&hash) {
        if existing != pattern {
            return Err(CBackendError::new(format!(
                "c backend pattern hash collision for hash {hash}"
            )));
        }
    } else {
        patterns.insert(hash, pattern.clone());
    }

    match pattern {
        IrPattern::Tuple { items } => {
            for item in items {
                register_pattern(item, patterns)?;
            }
        }
        IrPattern::List { items, tail } => {
            for item in items {
                register_pattern(item, patterns)?;
            }
            if let Some(tail_pattern) = tail {
                register_pattern(tail_pattern, patterns)?;
            }
        }
        IrPattern::Map { entries } => {
            for entry in entries {
                register_pattern(&entry.key, patterns)?;
                register_pattern(&entry.value, patterns)?;
            }
        }
        IrPattern::Atom { .. }
        | IrPattern::Bind { .. }
        | IrPattern::Pin { .. }
        | IrPattern::Wildcard
        | IrPattern::Integer { .. }
        | IrPattern::Bool { .. }
        | IrPattern::Nil
        | IrPattern::String { .. }
        | IrPattern::Bitstring { .. } => {}
    }

    Ok(())
}
