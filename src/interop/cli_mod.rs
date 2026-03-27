use super::system::expect_exact_args;
use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;

/// Extract a string from a RuntimeValue, or return None.
fn as_str(v: &RuntimeValue) -> Option<&str> {
    match v {
        RuntimeValue::String(s) => Some(s.as_str()),
        RuntimeValue::Atom(s) => Some(s.as_str()),
        _ => None,
    }
}

/// Extract keyword value by atom key from a keyword list.
fn kw_get<'a>(entries: &'a [(RuntimeValue, RuntimeValue)], key: &str) -> Option<&'a RuntimeValue> {
    entries.iter().find_map(|(k, v)| match k {
        RuntimeValue::Atom(s) if s == key => Some(v),
        _ => None,
    })
}

/// Extract keyword entries from a RuntimeValue::Keyword.
fn as_keyword(v: &RuntimeValue) -> Option<&Vec<(RuntimeValue, RuntimeValue)>> {
    match v {
        RuntimeValue::Keyword(entries) => Some(entries),
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct FlagSpec {
    name: String,
    flag_type: FlagType,
    short: Option<String>,
    doc: String,
    default: RuntimeValue,
    required: bool,
}

#[derive(Debug, Clone)]
struct ArgSpec {
    name: String,
    doc: String,
    required: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum FlagType {
    Boolean,
    String,
    Integer,
    Float,
}

#[derive(Debug)]
struct CliSpec {
    name: String,
    version: String,
    description: String,
    flags: Vec<FlagSpec>,
    args: Vec<ArgSpec>,
}

/// Parse the spec keyword list into a CliSpec struct.
fn parse_spec(spec_kw: &[(RuntimeValue, RuntimeValue)]) -> Result<CliSpec, HostError> {
    let name = kw_get(spec_kw, "name")
        .and_then(|v| as_str(v))
        .unwrap_or("app")
        .to_string();

    let version = kw_get(spec_kw, "version")
        .and_then(|v| as_str(v))
        .unwrap_or("0.1.0")
        .to_string();

    let description = kw_get(spec_kw, "description")
        .and_then(|v| as_str(v))
        .unwrap_or("")
        .to_string();

    let mut flags = Vec::new();
    if let Some(RuntimeValue::Keyword(flag_entries)) = kw_get(spec_kw, "flags") {
        for (key, val) in flag_entries {
            let flag_name = as_str(key).ok_or_else(|| {
                HostError::new("CLI.spec: flag name must be an atom or string".to_string())
            })?;
            let flag_kw = as_keyword(val).ok_or_else(|| {
                HostError::new(format!(
                    "CLI.spec: flag '{flag_name}' options must be a keyword list"
                ))
            })?;

            let flag_type = match kw_get(flag_kw, "type").and_then(|v| as_str(v)) {
                Some("boolean") => FlagType::Boolean,
                Some("string") => FlagType::String,
                Some("integer") => FlagType::Integer,
                Some("float") => FlagType::Float,
                Some(other) => {
                    return Err(HostError::new(format!(
                        "CLI.spec: unknown flag type '{other}' for flag '{flag_name}'"
                    )));
                }
                None => FlagType::String, // default type
            };

            let short = kw_get(flag_kw, "short")
                .and_then(|v| as_str(v))
                .map(|s| s.to_string());
            let doc = kw_get(flag_kw, "doc")
                .and_then(|v| as_str(v))
                .unwrap_or("")
                .to_string();
            let default = match &flag_type {
                FlagType::Boolean => kw_get(flag_kw, "default")
                    .cloned()
                    .unwrap_or(RuntimeValue::Bool(false)),
                _ => kw_get(flag_kw, "default")
                    .cloned()
                    .unwrap_or(RuntimeValue::Nil),
            };
            let required = matches!(kw_get(flag_kw, "required"), Some(RuntimeValue::Bool(true)));

            flags.push(FlagSpec {
                name: flag_name.to_string(),
                flag_type,
                short,
                doc,
                default,
                required,
            });
        }
    }

    let mut args = Vec::new();
    if let Some(RuntimeValue::Keyword(arg_entries)) = kw_get(spec_kw, "args") {
        for (key, val) in arg_entries {
            let arg_name = as_str(key).ok_or_else(|| {
                HostError::new("CLI.spec: arg name must be an atom or string".to_string())
            })?;
            let arg_kw = as_keyword(val).ok_or_else(|| {
                HostError::new(format!(
                    "CLI.spec: arg '{arg_name}' options must be a keyword list"
                ))
            })?;

            let doc = kw_get(arg_kw, "doc")
                .and_then(|v| as_str(v))
                .unwrap_or("")
                .to_string();
            let required = match kw_get(arg_kw, "required") {
                Some(RuntimeValue::Bool(true)) => true,
                _ => false,
            };

            args.push(ArgSpec {
                name: arg_name.to_string(),
                doc,
                required,
            });
        }
    }

    Ok(CliSpec {
        name,
        version,
        description,
        flags,
        args,
    })
}

/// Generate help text from a CliSpec.
fn generate_help(spec: &CliSpec) -> String {
    let mut lines = Vec::new();

    // Header
    if !spec.description.is_empty() {
        lines.push(format!(
            "{} v{} — {}",
            spec.name, spec.version, spec.description
        ));
    } else {
        lines.push(format!("{} v{}", spec.name, spec.version));
    }
    lines.push(String::new());

    // Usage line
    let mut usage = format!("USAGE: {}", spec.name);
    if !spec.flags.is_empty() {
        usage.push_str(" [OPTIONS]");
    }
    for arg in &spec.args {
        if arg.required {
            usage.push_str(&format!(" <{}>", arg.name));
        } else {
            usage.push_str(&format!(" [{}]", arg.name));
        }
    }
    lines.push(usage);

    // Args section
    if !spec.args.is_empty() {
        lines.push(String::new());
        lines.push("ARGS:".to_string());
        for arg in &spec.args {
            let req = if arg.required { " (required)" } else { "" };
            lines.push(format!("  <{}>{}    {}", arg.name, req, arg.doc));
        }
    }

    // Flags section (including built-in ones)
    lines.push(String::new());
    lines.push("OPTIONS:".to_string());

    for flag in &spec.flags {
        let mut flag_str = String::from("  ");
        if let Some(short) = &flag.short {
            flag_str.push_str(&format!("-{}, ", short));
        } else {
            flag_str.push_str("    ");
        }
        flag_str.push_str(&format!("--{}", flag.name));

        let type_hint = match &flag.flag_type {
            FlagType::Boolean => "",
            FlagType::String => " <string>",
            FlagType::Integer => " <integer>",
            FlagType::Float => " <float>",
        };
        flag_str.push_str(type_hint);

        if !flag.doc.is_empty() {
            // Pad to align descriptions
            let pad = 30usize.saturating_sub(flag_str.len());
            flag_str.push_str(&" ".repeat(pad));
            flag_str.push_str(&flag.doc);
        }

        lines.push(flag_str);
    }

    // Built-in flags
    lines.push("      --output-json           Output as JSON".to_string());
    lines.push("  -h, --help                  Show this help".to_string());
    lines.push("      --version               Show version".to_string());

    lines.join("\n")
}

/// Parse argv against a spec. Returns {:ok, result}, {:help, text}, {:version, text}, or {:error, msg}.
fn do_parse(spec: &CliSpec, argv: &[String]) -> RuntimeValue {
    let mut flag_values: Vec<(RuntimeValue, RuntimeValue)> = Vec::new();
    let mut positional: Vec<String> = Vec::new();
    let mut output_json = false;
    let mut i = 0;

    // Initialize flag defaults
    let mut parsed_flags: Vec<(String, RuntimeValue)> = spec
        .flags
        .iter()
        .map(|f| (f.name.clone(), f.default.clone()))
        .collect();

    while i < argv.len() {
        let arg = &argv[i];

        if arg == "--help" || arg == "-h" {
            let help = generate_help(spec);
            return RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("help".to_string())),
                Box::new(RuntimeValue::String(help)),
            );
        }

        if arg == "--version" {
            let ver = format!("{} v{}", spec.name, spec.version);
            return RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("version".to_string())),
                Box::new(RuntimeValue::String(ver)),
            );
        }

        if arg == "--output-json" {
            output_json = true;
            i += 1;
            continue;
        }

        if arg.starts_with("--") {
            let flag_name = &arg[2..];
            // Check for --no-<flag> boolean negation
            if let Some(stripped) = flag_name.strip_prefix("no-") {
                if let Some(fspec) = spec.flags.iter().find(|f| f.name == stripped) {
                    if fspec.flag_type == FlagType::Boolean {
                        if let Some(entry) = parsed_flags.iter_mut().find(|(n, _)| n == stripped) {
                            entry.1 = RuntimeValue::Bool(false);
                        }
                        i += 1;
                        continue;
                    }
                }
            }

            if let Some(fspec) = spec.flags.iter().find(|f| f.name == flag_name) {
                match &fspec.flag_type {
                    FlagType::Boolean => {
                        if let Some(entry) = parsed_flags.iter_mut().find(|(n, _)| n == flag_name) {
                            entry.1 = RuntimeValue::Bool(true);
                        }
                    }
                    _ => {
                        i += 1;
                        if i >= argv.len() {
                            return error_tuple(format!("flag --{flag_name} requires a value"));
                        }
                        match parse_flag_value(fspec, &argv[i]) {
                            Ok(val) => {
                                if let Some(entry) =
                                    parsed_flags.iter_mut().find(|(n, _)| n == flag_name)
                                {
                                    entry.1 = val;
                                }
                            }
                            Err(msg) => return error_tuple(msg),
                        }
                    }
                }
            } else {
                return error_tuple(format!("unknown flag --{flag_name}"));
            }
            i += 1;
            continue;
        }

        if arg.starts_with('-') && arg.len() == 2 {
            let short = &arg[1..2];
            if let Some(fspec) = spec
                .flags
                .iter()
                .find(|f| f.short.as_deref() == Some(short))
            {
                let flag_name = fspec.name.clone();
                match &fspec.flag_type {
                    FlagType::Boolean => {
                        if let Some(entry) = parsed_flags.iter_mut().find(|(n, _)| *n == flag_name)
                        {
                            entry.1 = RuntimeValue::Bool(true);
                        }
                    }
                    _ => {
                        i += 1;
                        if i >= argv.len() {
                            return error_tuple(format!(
                                "flag -{short} (--{flag_name}) requires a value"
                            ));
                        }
                        match parse_flag_value(fspec, &argv[i]) {
                            Ok(val) => {
                                if let Some(entry) =
                                    parsed_flags.iter_mut().find(|(n, _)| *n == flag_name)
                                {
                                    entry.1 = val;
                                }
                            }
                            Err(msg) => return error_tuple(msg),
                        }
                    }
                }
            } else {
                return error_tuple(format!("unknown flag -{short}"));
            }
            i += 1;
            continue;
        }

        // Positional argument
        positional.push(arg.clone());
        i += 1;
    }

    // Check required flags
    for fspec in &spec.flags {
        if fspec.required {
            if let Some((_, val)) = parsed_flags.iter().find(|(n, _)| *n == fspec.name) {
                if *val == RuntimeValue::Nil {
                    return error_tuple(format!("required flag --{} is missing", fspec.name));
                }
            }
        }
    }

    // Map positional args to arg specs
    let mut arg_values: Vec<(RuntimeValue, RuntimeValue)> = Vec::new();
    for (idx, aspec) in spec.args.iter().enumerate() {
        if idx < positional.len() {
            arg_values.push((
                RuntimeValue::Atom(aspec.name.clone()),
                RuntimeValue::String(positional[idx].clone()),
            ));
        } else if aspec.required {
            return error_tuple(format!("required argument <{}> is missing", aspec.name));
        } else {
            arg_values.push((RuntimeValue::Atom(aspec.name.clone()), RuntimeValue::Nil));
        }
    }

    // Collect extra positional args
    let rest_args: Vec<RuntimeValue> = positional
        .iter()
        .skip(spec.args.len())
        .map(|s| RuntimeValue::String(s.clone()))
        .collect();

    // Build flags map
    for (name, val) in &parsed_flags {
        flag_values.push((RuntimeValue::Atom(name.clone()), val.clone()));
    }

    // Build result map
    let result = RuntimeValue::Map(vec![
        (
            RuntimeValue::Atom("flags".to_string()),
            RuntimeValue::Map(flag_values),
        ),
        (
            RuntimeValue::Atom("args".to_string()),
            RuntimeValue::Map(arg_values),
        ),
        (
            RuntimeValue::Atom("rest".to_string()),
            RuntimeValue::List(rest_args),
        ),
        (
            RuntimeValue::Atom("output_json".to_string()),
            RuntimeValue::Bool(output_json),
        ),
    ]);

    RuntimeValue::Tuple(
        Box::new(RuntimeValue::Atom("ok".to_string())),
        Box::new(result),
    )
}

fn parse_flag_value(fspec: &FlagSpec, raw: &str) -> Result<RuntimeValue, String> {
    match &fspec.flag_type {
        FlagType::String => Ok(RuntimeValue::String(raw.to_string())),
        FlagType::Integer => raw
            .parse::<i64>()
            .map(RuntimeValue::Int)
            .map_err(|_| format!("flag --{} expects an integer, got '{}'", fspec.name, raw)),
        FlagType::Float => {
            // Accept integer strings as valid floats too
            if let Ok(i) = raw.parse::<i64>() {
                Ok(RuntimeValue::Float(format!("{}.0", i)))
            } else {
                raw.parse::<f64>()
                    .map(|_| RuntimeValue::Float(raw.to_string()))
                    .map_err(|_| format!("flag --{} expects a float, got '{}'", fspec.name, raw))
            }
        }
        FlagType::Boolean => {
            // Shouldn't be called for booleans, but handle gracefully
            Ok(RuntimeValue::Bool(true))
        }
    }
}

fn error_tuple(msg: String) -> RuntimeValue {
    RuntimeValue::Tuple(
        Box::new(RuntimeValue::Atom("error".to_string())),
        Box::new(RuntimeValue::String(msg)),
    )
}

/// host_cli_build_spec: takes a keyword list, validates it, returns it as-is (normalized).
/// In this design, the spec is passed through — validation happens at parse time.
fn host_cli_build_spec(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("CLI.spec", args, 1)?;
    // Just validate and return — the spec is a keyword list passed to parse
    match &args[0] {
        RuntimeValue::Keyword(_) => Ok(args[0].clone()),
        other => Err(HostError::new(format!(
            "CLI.spec expects a keyword list, found {}",
            host_value_kind(other)
        ))),
    }
}

/// host_cli_parse: takes a spec (keyword list) and argv (list of strings).
/// Returns {:ok, result}, {:help, text}, {:version, text}, or {:error, message}.
fn host_cli_parse(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("CLI.parse", args, 2)?;

    let spec_kw = match &args[0] {
        RuntimeValue::Keyword(entries) => entries,
        other => {
            return Err(HostError::new(format!(
                "CLI.parse expects a spec (keyword list) as first argument, found {}",
                host_value_kind(other)
            )));
        }
    };

    let argv_list = match &args[1] {
        RuntimeValue::List(items) => items,
        other => {
            return Err(HostError::new(format!(
                "CLI.parse expects a list of strings as second argument, found {}",
                host_value_kind(other)
            )));
        }
    };

    let spec = parse_spec(spec_kw)?;

    let argv: Vec<String> = argv_list
        .iter()
        .map(|v| match v {
            RuntimeValue::String(s) => Ok(s.clone()),
            other => Err(HostError::new(format!(
                "CLI.parse argv must contain strings, found {}",
                host_value_kind(other)
            ))),
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(do_parse(&spec, &argv))
}

/// host_cli_format_help: takes a spec, returns help text.
fn host_cli_format_help(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("CLI.help_text", args, 1)?;

    let spec_kw = match &args[0] {
        RuntimeValue::Keyword(entries) => entries,
        other => {
            return Err(HostError::new(format!(
                "CLI.help_text expects a spec (keyword list), found {}",
                host_value_kind(other)
            )));
        }
    };

    let spec = parse_spec(spec_kw)?;
    Ok(RuntimeValue::String(generate_help(&spec)))
}

pub fn register_cli_host_functions(registry: &HostRegistry) {
    registry.register("cli_build_spec", host_cli_build_spec);
    registry.register("cli_parse", host_cli_parse);
    registry.register("cli_format_help", host_cli_format_help);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(val: &str) -> RuntimeValue {
        RuntimeValue::Atom(val.to_string())
    }

    fn s(val: &str) -> RuntimeValue {
        RuntimeValue::String(val.to_string())
    }

    fn int(val: i64) -> RuntimeValue {
        RuntimeValue::Int(val)
    }

    fn kw(entries: Vec<(&str, RuntimeValue)>) -> RuntimeValue {
        RuntimeValue::Keyword(entries.into_iter().map(|(k, v)| (atom(k), v)).collect())
    }

    fn list(items: Vec<RuntimeValue>) -> RuntimeValue {
        RuntimeValue::List(items)
    }

    fn make_spec() -> RuntimeValue {
        kw(vec![
            ("name", s("myapp")),
            ("version", s("1.0.0")),
            ("description", s("A test CLI")),
            (
                "flags",
                kw(vec![
                    (
                        "verbose",
                        kw(vec![
                            ("type", atom("boolean")),
                            ("short", s("v")),
                            ("doc", s("Enable verbose output")),
                        ]),
                    ),
                    (
                        "count",
                        kw(vec![
                            ("type", atom("integer")),
                            ("short", s("n")),
                            ("default", int(1)),
                            ("doc", s("Repetitions")),
                        ]),
                    ),
                    (
                        "output",
                        kw(vec![
                            ("type", atom("string")),
                            ("short", s("o")),
                            ("doc", s("Output file")),
                        ]),
                    ),
                ]),
            ),
            (
                "args",
                kw(vec![(
                    "file",
                    kw(vec![
                        ("doc", s("Input file")),
                        ("required", RuntimeValue::Bool(true)),
                    ]),
                )]),
            ),
        ])
    }

    fn argv(args: &[&str]) -> RuntimeValue {
        list(args.iter().map(|a| s(a)).collect())
    }

    fn extract_ok(result: &RuntimeValue) -> &RuntimeValue {
        match result {
            RuntimeValue::Tuple(tag, val) => {
                assert_eq!(**tag, atom("ok"));
                val
            }
            _ => panic!("expected ok tuple, got {:?}", result),
        }
    }

    fn get_map_field<'a>(map: &'a RuntimeValue, key: &str) -> &'a RuntimeValue {
        match map {
            RuntimeValue::Map(entries) => {
                for (k, v) in entries {
                    if *k == atom(key) {
                        return v;
                    }
                }
                panic!("key '{}' not found in map", key);
            }
            _ => panic!("expected map"),
        }
    }

    // --- Tests ---

    #[test]
    fn cli_module_parse_basic_flags_and_args() {
        let spec = make_spec();
        let result = host_cli_parse(&[spec, argv(&["--verbose", "-n", "5", "input.txt"])]).unwrap();
        let data = extract_ok(&result);
        let flags = get_map_field(data, "flags");
        assert_eq!(*get_map_field(flags, "verbose"), RuntimeValue::Bool(true));
        assert_eq!(*get_map_field(flags, "count"), int(5));
        let args_map = get_map_field(data, "args");
        assert_eq!(*get_map_field(args_map, "file"), s("input.txt"));
    }

    #[test]
    fn cli_module_parse_defaults() {
        let spec = make_spec();
        let result = host_cli_parse(&[spec, argv(&["input.txt"])]).unwrap();
        let data = extract_ok(&result);
        let flags = get_map_field(data, "flags");
        assert_eq!(*get_map_field(flags, "verbose"), RuntimeValue::Bool(false));
        assert_eq!(*get_map_field(flags, "count"), int(1));
        assert_eq!(*get_map_field(flags, "output"), RuntimeValue::Nil);
    }

    #[test]
    fn cli_module_parse_help_flag() {
        let spec = make_spec();
        let result = host_cli_parse(&[spec, argv(&["--help"])]).unwrap();
        match &result {
            RuntimeValue::Tuple(tag, val) => {
                assert_eq!(**tag, atom("help"));
                if let RuntimeValue::String(text) = val.as_ref() {
                    assert!(text.contains("myapp"));
                    assert!(text.contains("USAGE:"));
                    assert!(text.contains("--verbose"));
                    assert!(text.contains("--output-json"));
                } else {
                    panic!("expected help text string");
                }
            }
            _ => panic!("expected help tuple"),
        }
    }

    #[test]
    fn cli_module_parse_short_help() {
        let spec = make_spec();
        let result = host_cli_parse(&[spec, argv(&["-h"])]).unwrap();
        match &result {
            RuntimeValue::Tuple(tag, _) => assert_eq!(**tag, atom("help")),
            _ => panic!("expected help tuple"),
        }
    }

    #[test]
    fn cli_module_parse_version_flag() {
        let spec = make_spec();
        let result = host_cli_parse(&[spec, argv(&["--version"])]).unwrap();
        match &result {
            RuntimeValue::Tuple(tag, val) => {
                assert_eq!(**tag, atom("version"));
                if let RuntimeValue::String(text) = val.as_ref() {
                    assert!(text.contains("myapp v1.0.0"));
                } else {
                    panic!("expected version text string");
                }
            }
            _ => panic!("expected version tuple"),
        }
    }

    #[test]
    fn cli_module_parse_output_json_flag() {
        let spec = make_spec();
        let result = host_cli_parse(&[spec, argv(&["--output-json", "input.txt"])]).unwrap();
        let data = extract_ok(&result);
        assert_eq!(
            *get_map_field(data, "output_json"),
            RuntimeValue::Bool(true)
        );
    }

    #[test]
    fn cli_module_parse_missing_required_arg() {
        let spec = make_spec();
        let result = host_cli_parse(&[spec, argv(&["--verbose"])]).unwrap();
        match &result {
            RuntimeValue::Tuple(tag, val) => {
                assert_eq!(**tag, atom("error"));
                if let RuntimeValue::String(msg) = val.as_ref() {
                    assert!(msg.contains("file"));
                    assert!(msg.contains("missing"));
                } else {
                    panic!("expected error message string");
                }
            }
            _ => panic!("expected error tuple"),
        }
    }

    #[test]
    fn cli_module_parse_unknown_flag() {
        let spec = make_spec();
        let result = host_cli_parse(&[spec, argv(&["--unknown", "input.txt"])]).unwrap();
        match &result {
            RuntimeValue::Tuple(tag, val) => {
                assert_eq!(**tag, atom("error"));
                if let RuntimeValue::String(msg) = val.as_ref() {
                    assert!(msg.contains("unknown"));
                } else {
                    panic!("expected error message string");
                }
            }
            _ => panic!("expected error tuple"),
        }
    }

    #[test]
    fn cli_module_parse_integer_validation() {
        let spec = make_spec();
        let result = host_cli_parse(&[spec, argv(&["--count", "abc", "input.txt"])]).unwrap();
        match &result {
            RuntimeValue::Tuple(tag, val) => {
                assert_eq!(**tag, atom("error"));
                if let RuntimeValue::String(msg) = val.as_ref() {
                    assert!(msg.contains("integer"));
                } else {
                    panic!("expected error message string");
                }
            }
            _ => panic!("expected error tuple"),
        }
    }

    #[test]
    fn cli_module_parse_short_flags() {
        let spec = make_spec();
        let result = host_cli_parse(&[spec, argv(&["-v", "-o", "out.txt", "in.txt"])]).unwrap();
        let data = extract_ok(&result);
        let flags = get_map_field(data, "flags");
        assert_eq!(*get_map_field(flags, "verbose"), RuntimeValue::Bool(true));
        assert_eq!(*get_map_field(flags, "output"), s("out.txt"));
    }

    #[test]
    fn cli_module_parse_rest_args() {
        let spec = make_spec();
        let result = host_cli_parse(&[spec, argv(&["input.txt", "extra1", "extra2"])]).unwrap();
        let data = extract_ok(&result);
        let rest = get_map_field(data, "rest");
        assert_eq!(*rest, list(vec![s("extra1"), s("extra2")]));
    }

    #[test]
    fn cli_module_parse_missing_flag_value() {
        let spec = make_spec();
        let result = host_cli_parse(&[spec, argv(&["input.txt", "--count"])]).unwrap();
        match &result {
            RuntimeValue::Tuple(tag, val) => {
                assert_eq!(**tag, atom("error"));
                if let RuntimeValue::String(msg) = val.as_ref() {
                    assert!(msg.contains("requires a value"));
                } else {
                    panic!("expected error message string");
                }
            }
            _ => panic!("expected error tuple"),
        }
    }

    #[test]
    fn cli_module_parse_boolean_no_prefix() {
        let spec = make_spec();
        let result =
            host_cli_parse(&[spec, argv(&["--verbose", "--no-verbose", "input.txt"])]).unwrap();
        let data = extract_ok(&result);
        let flags = get_map_field(data, "flags");
        assert_eq!(*get_map_field(flags, "verbose"), RuntimeValue::Bool(false));
    }

    #[test]
    fn cli_module_build_spec_validates_keyword() {
        let result = host_cli_build_spec(&[RuntimeValue::Int(42)]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_module_build_spec_accepts_keyword() {
        let spec = make_spec();
        let result = host_cli_build_spec(&[spec.clone()]).unwrap();
        assert_eq!(result, spec);
    }

    #[test]
    fn cli_module_help_text_generation() {
        let spec = make_spec();
        let result = host_cli_format_help(&[spec]).unwrap();
        if let RuntimeValue::String(text) = result {
            assert!(text.contains("myapp v1.0.0"));
            assert!(text.contains("A test CLI"));
            assert!(text.contains("USAGE:"));
            assert!(text.contains("<file>"));
            assert!(text.contains("--verbose"));
            assert!(text.contains("-v"));
            assert!(text.contains("--output-json"));
            assert!(text.contains("--help"));
            assert!(text.contains("--version"));
        } else {
            panic!("expected string");
        }
    }

    #[test]
    fn cli_module_parse_float_flag() {
        let spec = kw(vec![
            ("name", s("app")),
            (
                "flags",
                kw(vec![("threshold", kw(vec![("type", atom("float"))]))]),
            ),
        ]);
        let result = host_cli_parse(&[spec, argv(&["--threshold", "0.5"])]).unwrap();
        let data = extract_ok(&result);
        let flags = get_map_field(data, "flags");
        assert_eq!(
            *get_map_field(flags, "threshold"),
            RuntimeValue::Float("0.5".to_string())
        );
    }

    #[test]
    fn cli_module_parse_required_flag() {
        let spec = kw(vec![
            ("name", s("app")),
            (
                "flags",
                kw(vec![(
                    "token",
                    kw(vec![
                        ("type", atom("string")),
                        ("required", RuntimeValue::Bool(true)),
                    ]),
                )]),
            ),
        ]);
        let result = host_cli_parse(&[spec, argv(&[])]).unwrap();
        match &result {
            RuntimeValue::Tuple(tag, val) => {
                assert_eq!(**tag, atom("error"));
                if let RuntimeValue::String(msg) = val.as_ref() {
                    assert!(msg.contains("token"));
                    assert!(msg.contains("missing"));
                } else {
                    panic!("expected error message string");
                }
            }
            _ => panic!("expected error tuple"),
        }
    }

    #[test]
    fn cli_module_parse_empty_spec() {
        let spec = kw(vec![("name", s("minimal"))]);
        let result = host_cli_parse(&[spec, argv(&[])]).unwrap();
        let data = extract_ok(&result);
        assert_eq!(
            *get_map_field(data, "output_json"),
            RuntimeValue::Bool(false)
        );
    }

    #[test]
    fn cli_module_registration() {
        let registry = HostRegistry::new();
        register_cli_host_functions(&registry);
        let spec = make_spec();
        let result = registry
            .call("cli_parse", &[spec, argv(&["input.txt"])])
            .unwrap();
        let data = extract_ok(&result);
        let args_map = get_map_field(data, "args");
        assert_eq!(*get_map_field(args_map, "file"), s("input.txt"));
    }
}
