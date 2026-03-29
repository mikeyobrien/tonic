use crate::mir::MirInstruction;

pub(crate) fn mangle_function_name(name: &str, arity: usize) -> String {
    format!("tn_{}__arity{arity}", sanitize_identifier(name))
}

fn sanitize_identifier(input: &str) -> String {
    input
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

pub(crate) fn instruction_name(instruction: &MirInstruction) -> &'static str {
    match instruction {
        MirInstruction::ConstInt { .. } => "const_int",
        MirInstruction::ConstFloat { .. } => "const_float",
        MirInstruction::ConstBool { .. } => "const_bool",
        MirInstruction::ConstNil { .. } => "const_nil",
        MirInstruction::ConstString { .. } => "const_string",
        MirInstruction::ConstAtom { .. } => "const_atom",
        MirInstruction::LoadVariable { .. } => "load_variable",
        MirInstruction::Unary { .. } => "unary",
        MirInstruction::Binary { .. } => "binary",
        MirInstruction::Call { .. } => "call",
        MirInstruction::CallValue { .. } => "call_value",
        MirInstruction::MakeClosure { .. } => "make_closure",
        MirInstruction::Question { .. } => "question",
        MirInstruction::MatchPattern { .. } => "match_pattern",
        MirInstruction::Legacy { .. } => "legacy",
    }
}
