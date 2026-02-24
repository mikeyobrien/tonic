use crate::ir::IrPattern;
use crate::mir::{MirInstruction, MirProgram, MirTerminator};
use std::collections::BTreeMap;

use super::error::CBackendError;
use super::hash::{hash_pattern_i64, hash_text_i64};

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
                if let MirInstruction::MatchPattern { pattern, .. } = instruction {
                    register_pattern(pattern, &mut patterns)?;
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
        | IrPattern::String { .. } => {}
    }

    Ok(())
}

fn emit_pattern_case(pattern_case: &PatternCase, out: &mut String) -> Result<(), CBackendError> {
    out.push_str(&format!(
        "static int {}(TnVal value) {{\n",
        pattern_case.symbol
    ));

    match &pattern_case.pattern {
        IrPattern::Wildcard => {
            out.push_str("  return 1;\n");
        }
        IrPattern::Bind { name } => {
            let name_hash = hash_text_i64(name);
            out.push_str("  TnVal existing = 0;\n");
            out.push_str(&format!("  TnVal key = (TnVal){name_hash}LL;\n"));
            out.push_str("  if (tn_binding_get(key, &existing)) {\n");
            out.push_str("    return tn_runtime_value_equal(existing, value);\n");
            out.push_str("  }\n");
            out.push('\n');
            out.push_str("  tn_binding_set(key, value);\n");
            out.push_str("  return 1;\n");
        }
        IrPattern::Pin { name } => {
            let name_hash = hash_text_i64(name);
            out.push_str("  TnVal pinned = 0;\n");
            out.push_str(&format!(
                "  if (!tn_binding_get((TnVal){name_hash}LL, &pinned)) {{\n"
            ));
            out.push_str("    return 0;\n");
            out.push_str("  }\n");
            out.push('\n');
            out.push_str("  return tn_runtime_value_equal(pinned, value);\n");
        }
        IrPattern::Integer { value } => {
            out.push_str("  if (tn_is_boxed(value)) {\n");
            out.push_str("    return 0;\n");
            out.push_str("  }\n");
            out.push_str(&format!("  return value == (TnVal){value}LL;\n"));
        }
        IrPattern::Bool { value } => {
            out.push_str(&format!(
                "  return tn_runtime_value_equal(value, tn_runtime_const_bool((TnVal){}));\n",
                if *value { 1 } else { 0 }
            ));
        }
        IrPattern::Nil => {
            out.push_str("  return tn_runtime_value_equal(value, tn_runtime_const_nil());\n");
        }
        IrPattern::String { value } => {
            let escaped = c_string_literal(value);
            out.push_str(&format!(
                "  return tn_runtime_value_equal(value, tn_runtime_const_string((TnVal)(intptr_t){escaped}));\n"
            ));
        }
        IrPattern::Atom { value } => {
            let escaped = c_string_literal(value);
            out.push_str(&format!(
                "  return tn_runtime_value_equal(value, tn_runtime_const_atom((TnVal)(intptr_t){escaped}));\n"
            ));
        }
        IrPattern::Tuple { items } => {
            out.push_str("  TnObj *tuple_obj = tn_get_obj(value);\n");
            out.push_str("  if (tuple_obj == NULL || tuple_obj->kind != TN_OBJ_TUPLE) {\n");
            out.push_str("    return 0;\n");
            out.push_str("  }\n");

            if items.len() != 2 {
                out.push_str("  return 0;\n");
            } else {
                let left_hash = hash_pattern_i64(&items[0])?;
                let right_hash = hash_pattern_i64(&items[1])?;
                out.push_str(&format!(
                    "  if (!tn_pattern_match_internal(tuple_obj->as.tuple.left, (TnVal){left_hash}LL)) {{\n"
                ));
                out.push_str("    return 0;\n");
                out.push_str("  }\n");
                out.push_str(&format!(
                    "  if (!tn_pattern_match_internal(tuple_obj->as.tuple.right, (TnVal){right_hash}LL)) {{\n"
                ));
                out.push_str("    return 0;\n");
                out.push_str("  }\n");
                out.push_str("  return 1;\n");
            }
        }
        IrPattern::List { items, tail } => {
            out.push_str("  TnObj *list_obj = tn_get_obj(value);\n");
            out.push_str("  if (list_obj == NULL || list_obj->kind != TN_OBJ_LIST) {\n");
            out.push_str("    return 0;\n");
            out.push_str("  }\n");
            out.push_str(&format!(
                "  if (list_obj->as.list.len < {}) {{\n",
                items.len()
            ));
            out.push_str("    return 0;\n");
            out.push_str("  }\n");

            for (index, item) in items.iter().enumerate() {
                let item_hash = hash_pattern_i64(item)?;
                out.push_str(&format!(
                    "  if (!tn_pattern_match_internal(list_obj->as.list.items[{index}], (TnVal){item_hash}LL)) {{\n"
                ));
                out.push_str("    return 0;\n");
                out.push_str("  }\n");
            }

            if let Some(tail_pattern) = tail {
                let tail_hash = hash_pattern_i64(tail_pattern)?;
                out.push_str(&format!(
                    "  size_t tail_len = list_obj->as.list.len - {};\n",
                    items.len()
                ));
                out.push_str("  TnObj *tail_obj = tn_new_obj(TN_OBJ_LIST);\n");
                out.push_str("  tail_obj->as.list.len = tail_len;\n");
                out.push_str(
                    "  tail_obj->as.list.items = tail_len == 0 ? NULL : (TnVal *)calloc(tail_len, sizeof(TnVal));\n",
                );
                out.push_str("  if (tail_len > 0 && tail_obj->as.list.items == NULL) {\n");
                out.push_str(
                    "    fprintf(stderr, \"error: native runtime allocation failure\\n\");\n",
                );
                out.push_str("    exit(1);\n");
                out.push_str("  }\n");
                out.push_str("  for (size_t i = 0; i < tail_len; i += 1) {\n");
                out.push_str(&format!(
                    "    tail_obj->as.list.items[i] = list_obj->as.list.items[{} + i];\n",
                    items.len()
                ));
                out.push_str("  }\n");
                out.push_str("  TnVal tail_value = tn_heap_store(tail_obj);\n");
                out.push_str(&format!(
                    "  return tn_pattern_match_internal(tail_value, (TnVal){tail_hash}LL);\n"
                ));
            } else {
                out.push_str(&format!(
                    "  return list_obj->as.list.len == {};\n",
                    items.len()
                ));
            }
        }
        IrPattern::Map { entries } => {
            out.push_str("  TnObj *map_obj = tn_get_obj(value);\n");
            out.push_str("  if (map_obj == NULL || map_obj->kind != TN_OBJ_MAP) {\n");
            out.push_str("    return 0;\n");
            out.push_str("  }\n");

            for (index, entry) in entries.iter().enumerate() {
                let key_hash = hash_pattern_i64(&entry.key)?;
                let value_hash = hash_pattern_i64(&entry.value)?;
                out.push_str(&format!("  int entry_matched_{index} = 0;\n"));
                out.push_str(&format!(
                    "  for (size_t candidate_{index} = 0; candidate_{index} < map_obj->as.map_like.len; candidate_{index} += 1) {{\n"
                ));
                out.push_str(&format!(
                    "    TnBinding snapshot_{index}[TN_MAX_BINDINGS];\n"
                ));
                out.push_str(&format!("    size_t snapshot_len_{index} = 0;\n"));
                out.push_str(&format!(
                    "    tn_binding_snapshot(snapshot_{index}, &snapshot_len_{index});\n"
                ));
                out.push_str(&format!(
                    "    if (tn_pattern_match_internal(map_obj->as.map_like.items[candidate_{index}].key, (TnVal){key_hash}LL) &&\n"
                ));
                out.push_str(&format!(
                    "        tn_pattern_match_internal(map_obj->as.map_like.items[candidate_{index}].value, (TnVal){value_hash}LL)) {{\n"
                ));
                out.push_str(&format!("      entry_matched_{index} = 1;\n"));
                out.push_str("      break;\n");
                out.push_str("    }\n");
                out.push_str(&format!(
                    "    tn_binding_restore(snapshot_{index}, snapshot_len_{index});\n"
                ));
                out.push_str("  }\n");
                out.push_str(&format!("  if (!entry_matched_{index}) {{\n"));
                out.push_str("    return 0;\n");
                out.push_str("  }\n");
            }

            out.push_str("  return 1;\n");
        }
    }

    out.push_str("}\n\n");
    Ok(())
}

fn c_string_literal(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}
