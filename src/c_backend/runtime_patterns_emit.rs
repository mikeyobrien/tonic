use super::super::error::CBackendError;
use super::super::hash::{hash_pattern_i64, hash_text_i64};
use super::super::stubs::c_string_literal;
use super::PatternCase;
use crate::ir::IrPattern;

pub(super) fn emit_pattern_case(
    pattern_case: &PatternCase,
    out: &mut String,
) -> Result<(), CBackendError> {
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
        IrPattern::Bitstring { segments } => {
            out.push_str("  TnObj *list_obj = tn_get_obj(value);\n");
            out.push_str("  if (list_obj == NULL || list_obj->kind != TN_OBJ_BINARY) {\n");
            out.push_str("    return 0;\n");
            out.push_str("  }\n");
            out.push_str(&format!(
                "  if (list_obj->as.list.len != {}) {{\n",
                segments.len()
            ));
            out.push_str("    return 0;\n");
            out.push_str("  }\n");

            for (index, segment) in segments.iter().enumerate() {
                match segment {
                    crate::ir::IrBitstringSegment::Wildcard => {}
                    crate::ir::IrBitstringSegment::Literal { value } => {
                        out.push_str(&format!(
                            "  if (list_obj->as.list.items[{index}] != (TnVal){value}LL) {{\n"
                        ));
                        out.push_str("    return 0;\n");
                        out.push_str("  }\n");
                    }
                    crate::ir::IrBitstringSegment::Bind { name } => {
                        let name_hash = hash_text_i64(name);
                        out.push_str("  {\n");
                        out.push_str(&format!(
                            "    TnVal bs_byte_{index} = list_obj->as.list.items[{index}];\n"
                        ));
                        out.push_str(&format!("    TnVal existing_{index} = 0;\n"));
                        out.push_str(&format!(
                            "    if (tn_binding_get((TnVal){name_hash}LL, &existing_{index})) {{\n"
                        ));
                        out.push_str(&format!(
                            "      if (!tn_runtime_value_equal(existing_{index}, bs_byte_{index})) {{\n"
                        ));
                        out.push_str("        return 0;\n");
                        out.push_str("      }\n");
                        out.push_str("    } else {\n");
                        out.push_str(&format!(
                            "      tn_binding_set((TnVal){name_hash}LL, bs_byte_{index});\n"
                        ));
                        out.push_str("    }\n");
                        out.push_str("  }\n");
                    }
                }
            }

            out.push_str("  return 1;\n");
        }
    }

    out.push_str("}\n\n");
    Ok(())
}
