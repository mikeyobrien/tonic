use crate::runtime::RuntimeValue;

pub(crate) const GUARD_BUILTIN_ARITY: usize = 1;

const GUARD_BUILTINS: [GuardBuiltinSpec; 9] = [
    GuardBuiltinSpec::new("is_integer", "tn_runtime_guard_is_integer"),
    GuardBuiltinSpec::new("is_float", "tn_runtime_guard_is_float"),
    GuardBuiltinSpec::new("is_number", "tn_runtime_guard_is_number"),
    GuardBuiltinSpec::new("is_atom", "tn_runtime_guard_is_atom"),
    GuardBuiltinSpec::new("is_binary", "tn_runtime_guard_is_binary"),
    GuardBuiltinSpec::new("is_list", "tn_runtime_guard_is_list"),
    GuardBuiltinSpec::new("is_tuple", "tn_runtime_guard_is_tuple"),
    GuardBuiltinSpec::new("is_map", "tn_runtime_guard_is_map"),
    GuardBuiltinSpec::new("is_nil", "tn_runtime_guard_is_nil"),
];

#[derive(Debug, Clone, Copy)]
struct GuardBuiltinSpec {
    name: &'static str,
    c_helper: &'static str,
}

impl GuardBuiltinSpec {
    const fn new(name: &'static str, c_helper: &'static str) -> Self {
        Self { name, c_helper }
    }
}

pub(crate) fn is_guard_builtin(name: &str) -> bool {
    GUARD_BUILTINS.iter().any(|builtin| builtin.name == name)
}

pub(crate) fn guard_builtin_arity(name: &str) -> Option<usize> {
    is_guard_builtin(name).then_some(GUARD_BUILTIN_ARITY)
}

pub(crate) fn evaluate_guard_builtin(name: &str, value: &RuntimeValue) -> Option<bool> {
    let result = match name {
        "is_integer" => matches!(value, RuntimeValue::Int(_)),
        "is_float" => matches!(value, RuntimeValue::Float(_)),
        "is_number" => matches!(value, RuntimeValue::Int(_) | RuntimeValue::Float(_)),
        "is_atom" => matches!(value, RuntimeValue::Atom(_)),
        "is_binary" => matches!(value, RuntimeValue::String(_)),
        "is_list" => matches!(value, RuntimeValue::List(_) | RuntimeValue::Keyword(_)),
        "is_tuple" => matches!(value, RuntimeValue::Tuple(_, _)),
        "is_map" => matches!(value, RuntimeValue::Map(_)),
        "is_nil" => matches!(value, RuntimeValue::Nil),
        _ => return None,
    };

    Some(result)
}

pub(crate) fn c_helper_name(name: &str) -> Option<&'static str> {
    GUARD_BUILTINS
        .iter()
        .find(|builtin| builtin.name == name)
        .map(|builtin| builtin.c_helper)
}

pub(crate) fn llvm_helper_name(name: &str) -> Option<&'static str> {
    c_helper_name(name)
}
