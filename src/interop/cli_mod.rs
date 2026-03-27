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

/// Parse choices from a flag keyword list — expects `choices: ["a", "b", "c"]`.
fn parse_choices(flag_kw: &[(RuntimeValue, RuntimeValue)]) -> Vec<String> {
    match kw_get(flag_kw, "choices") {
        Some(RuntimeValue::List(items)) => items
            .iter()
            .filter_map(|v| as_str(v).map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    }
}

/// Validate a parsed value against choices constraint.
fn validate_choices(flag_name: &str, value: &str, choices: &[String]) -> Result<(), String> {
    if choices.is_empty() {
        return Ok(());
    }
    if choices.iter().any(|c| c == value) {
        Ok(())
    } else {
        Err(format!(
            "invalid value '{}' for --{}, expected one of: {}",
            value,
            flag_name,
            choices.join(", ")
        ))
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
    choices: Vec<String>,
    env: Option<String>,
    multi: bool,
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

#[derive(Debug, Clone)]
struct SubcommandSpec {
    name: String,
    description: String,
    flags: Vec<FlagSpec>,
    args: Vec<ArgSpec>,
}

#[derive(Debug)]
struct CliSpec {
    name: String,
    version: String,
    description: String,
    flags: Vec<FlagSpec>,
    args: Vec<ArgSpec>,
    commands: Vec<SubcommandSpec>,
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
            let choices = parse_choices(flag_kw);
            let env = kw_get(flag_kw, "env")
                .and_then(|v| as_str(v))
                .map(|s| s.to_string());
            let multi = matches!(kw_get(flag_kw, "multi"), Some(RuntimeValue::Bool(true)));

            flags.push(FlagSpec {
                name: flag_name.to_string(),
                flag_type,
                short,
                doc,
                default,
                required,
                choices,
                env,
                multi,
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

    let mut commands = Vec::new();
    if let Some(RuntimeValue::Keyword(cmd_entries)) = kw_get(spec_kw, "commands") {
        for (key, val) in cmd_entries {
            let cmd_name = as_str(key).ok_or_else(|| {
                HostError::new("CLI.spec: command name must be an atom or string".to_string())
            })?;
            let cmd_kw = as_keyword(val).ok_or_else(|| {
                HostError::new(format!(
                    "CLI.spec: command '{cmd_name}' options must be a keyword list"
                ))
            })?;

            let cmd_desc = kw_get(cmd_kw, "description")
                .and_then(|v| as_str(v))
                .unwrap_or("")
                .to_string();

            let mut cmd_flags = Vec::new();
            if let Some(RuntimeValue::Keyword(flag_entries)) = kw_get(cmd_kw, "flags") {
                for (fkey, fval) in flag_entries {
                    let flag_name = as_str(fkey).ok_or_else(|| {
                        HostError::new(format!(
                            "CLI.spec: flag name in command '{cmd_name}' must be an atom or string"
                        ))
                    })?;
                    let flag_kw = as_keyword(fval).ok_or_else(|| {
                        HostError::new(format!(
                            "CLI.spec: flag '{flag_name}' in command '{cmd_name}' must be a keyword list"
                        ))
                    })?;

                    let flag_type = match kw_get(flag_kw, "type").and_then(|v| as_str(v)) {
                        Some("boolean") => FlagType::Boolean,
                        Some("string") => FlagType::String,
                        Some("integer") => FlagType::Integer,
                        Some("float") => FlagType::Float,
                        Some(other) => {
                            return Err(HostError::new(format!(
                                "CLI.spec: unknown flag type '{other}' in command '{cmd_name}'"
                            )));
                        }
                        None => FlagType::String,
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
                    let required =
                        matches!(kw_get(flag_kw, "required"), Some(RuntimeValue::Bool(true)));
                    let choices = parse_choices(flag_kw);
                    let env = kw_get(flag_kw, "env")
                        .and_then(|v| as_str(v))
                        .map(|s| s.to_string());
                    let multi = matches!(kw_get(flag_kw, "multi"), Some(RuntimeValue::Bool(true)));

                    cmd_flags.push(FlagSpec {
                        name: flag_name.to_string(),
                        flag_type,
                        short,
                        doc,
                        default,
                        required,
                        choices,
                        env,
                        multi,
                    });
                }
            }

            let mut cmd_args = Vec::new();
            if let Some(RuntimeValue::Keyword(arg_entries)) = kw_get(cmd_kw, "args") {
                for (akey, aval) in arg_entries {
                    let arg_name = as_str(akey).ok_or_else(|| {
                        HostError::new(format!(
                            "CLI.spec: arg name in command '{cmd_name}' must be an atom or string"
                        ))
                    })?;
                    let arg_kw = as_keyword(aval).ok_or_else(|| {
                        HostError::new(format!(
                            "CLI.spec: arg '{arg_name}' in command '{cmd_name}' must be a keyword list"
                        ))
                    })?;

                    let doc = kw_get(arg_kw, "doc")
                        .and_then(|v| as_str(v))
                        .unwrap_or("")
                        .to_string();
                    let required =
                        matches!(kw_get(arg_kw, "required"), Some(RuntimeValue::Bool(true)));

                    cmd_args.push(ArgSpec {
                        name: arg_name.to_string(),
                        doc,
                        required,
                    });
                }
            }

            commands.push(SubcommandSpec {
                name: cmd_name.to_string(),
                description: cmd_desc,
                flags: cmd_flags,
                args: cmd_args,
            });
        }
    }

    Ok(CliSpec {
        name,
        version,
        description,
        flags,
        args,
        commands,
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
    if !spec.commands.is_empty() {
        usage.push_str(" <COMMAND>");
    }
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

    // Commands section
    if !spec.commands.is_empty() {
        lines.push(String::new());
        lines.push("COMMANDS:".to_string());
        for cmd in &spec.commands {
            let mut cmd_line = format!("  {}", cmd.name);
            if !cmd.description.is_empty() {
                let pad = 20usize.saturating_sub(cmd_line.len());
                cmd_line.push_str(&" ".repeat(pad));
                cmd_line.push_str(&cmd.description);
            }
            lines.push(cmd_line);
        }
    }

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
        lines.push(format_flag_help(flag));
    }

    // Built-in flags
    lines.push("      --output-json           Output as JSON".to_string());
    lines.push("  -h, --help                  Show this help".to_string());
    lines.push("      --version               Show version".to_string());

    lines.join("\n")
}

/// Format a single flag's help line, including choices, env, and multi annotations.
fn format_flag_help(flag: &FlagSpec) -> String {
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

    // Build description parts
    let mut desc_parts: Vec<String> = Vec::new();
    if !flag.doc.is_empty() {
        desc_parts.push(flag.doc.clone());
    }
    if !flag.choices.is_empty() {
        desc_parts.push(format!("({})", flag.choices.join(", ")));
    }
    if let Some(ref env_name) = flag.env {
        desc_parts.push(format!("(env: {})", env_name));
    }
    if flag.multi {
        desc_parts.push("(can be repeated)".to_string());
    }

    if !desc_parts.is_empty() {
        let pad = 30usize.saturating_sub(flag_str.len());
        flag_str.push_str(&" ".repeat(pad));
        flag_str.push_str(&desc_parts.join(" "));
    }

    flag_str
}

/// Generate help text for a specific subcommand.
fn generate_command_help(spec: &CliSpec, cmd: &SubcommandSpec) -> String {
    let mut lines = Vec::new();

    // Header
    if !cmd.description.is_empty() {
        lines.push(format!("{} {} — {}", spec.name, cmd.name, cmd.description));
    } else {
        lines.push(format!("{} {}", spec.name, cmd.name));
    }
    lines.push(String::new());

    // Usage line
    let mut usage = format!("USAGE: {} {}", spec.name, cmd.name);
    if !cmd.flags.is_empty() {
        usage.push_str(" [OPTIONS]");
    }
    for arg in &cmd.args {
        if arg.required {
            usage.push_str(&format!(" <{}>", arg.name));
        } else {
            usage.push_str(&format!(" [{}]", arg.name));
        }
    }
    lines.push(usage);

    // Args section
    if !cmd.args.is_empty() {
        lines.push(String::new());
        lines.push("ARGS:".to_string());
        for arg in &cmd.args {
            let req = if arg.required { " (required)" } else { "" };
            lines.push(format!("  <{}>{}    {}", arg.name, req, arg.doc));
        }
    }

    // Flags section
    lines.push(String::new());
    lines.push("OPTIONS:".to_string());

    for flag in &cmd.flags {
        lines.push(format_flag_help(flag));
    }

    // Built-in flags
    lines.push("      --output-json           Output as JSON".to_string());
    lines.push("  -h, --help                  Show this help".to_string());

    lines.join("\n")
}

/// Parse argv for a subcommand context (after the subcommand name has been consumed).
fn do_parse_command(spec: &CliSpec, cmd: &SubcommandSpec, argv: &[String]) -> RuntimeValue {
    let mut positional: Vec<String> = Vec::new();
    let mut output_json = false;
    let mut i = 0;

    let mut parsed_flags = init_parsed_flags(&cmd.flags);

    while i < argv.len() {
        let arg = &argv[i];

        if arg == "--help" || arg == "-h" {
            let help = generate_command_help(spec, cmd);
            return RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("help".to_string())),
                Box::new(RuntimeValue::String(help)),
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
                if let Some(fspec) = cmd.flags.iter().find(|f| f.name == stripped) {
                    if fspec.flag_type == FlagType::Boolean {
                        set_flag_value(
                            &mut parsed_flags,
                            stripped,
                            RuntimeValue::Bool(false),
                            false,
                        );
                        i += 1;
                        continue;
                    }
                }
            }

            if let Some(fspec) = cmd.flags.iter().find(|f| f.name == flag_name) {
                match &fspec.flag_type {
                    FlagType::Boolean => {
                        set_flag_value(
                            &mut parsed_flags,
                            flag_name,
                            RuntimeValue::Bool(true),
                            false,
                        );
                    }
                    _ => {
                        i += 1;
                        if i >= argv.len() {
                            return error_tuple(format!("flag --{flag_name} requires a value"));
                        }
                        match parse_flag_value(fspec, &argv[i]) {
                            Ok(val) => {
                                set_flag_value(&mut parsed_flags, flag_name, val, fspec.multi);
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
            if let Some(fspec) = cmd.flags.iter().find(|f| f.short.as_deref() == Some(short)) {
                let flag_name = fspec.name.clone();
                let is_multi = fspec.multi;
                match &fspec.flag_type {
                    FlagType::Boolean => {
                        set_flag_value(
                            &mut parsed_flags,
                            &flag_name,
                            RuntimeValue::Bool(true),
                            false,
                        );
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
                                set_flag_value(&mut parsed_flags, &flag_name, val, is_multi);
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

        positional.push(arg.clone());
        i += 1;
    }

    // Apply env fallback
    if let Err(msg) = apply_env_fallback(&cmd.flags, &mut parsed_flags) {
        return error_tuple(msg);
    }

    // Check required flags
    for fspec in &cmd.flags {
        if fspec.required {
            if let Some((_, val)) = parsed_flags.iter().find(|(n, _)| *n == fspec.name) {
                if *val == RuntimeValue::Nil {
                    return error_tuple(format!("required flag --{} is missing", fspec.name));
                }
            }
        }
    }

    // Map positional args
    let mut arg_values: Vec<(RuntimeValue, RuntimeValue)> = Vec::new();
    for (idx, aspec) in cmd.args.iter().enumerate() {
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

    let rest_args: Vec<RuntimeValue> = positional
        .iter()
        .skip(cmd.args.len())
        .map(|s| RuntimeValue::String(s.clone()))
        .collect();

    let mut flag_values: Vec<(RuntimeValue, RuntimeValue)> = Vec::new();
    for (name, val) in &parsed_flags {
        flag_values.push((RuntimeValue::Atom(name.clone()), val.clone()));
    }

    let result = RuntimeValue::Map(vec![
        (
            RuntimeValue::Atom("command".to_string()),
            RuntimeValue::String(cmd.name.clone()),
        ),
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

/// Initialize parsed_flags with defaults, respecting multi flags (default []).
fn init_parsed_flags(flags: &[FlagSpec]) -> Vec<(String, RuntimeValue)> {
    flags
        .iter()
        .map(|f| {
            let default = if f.multi {
                RuntimeValue::List(vec![])
            } else {
                f.default.clone()
            };
            (f.name.clone(), default)
        })
        .collect()
}

/// Set a flag value, handling multi (append) vs single (replace).
fn set_flag_value(
    parsed_flags: &mut [(String, RuntimeValue)],
    flag_name: &str,
    val: RuntimeValue,
    multi: bool,
) {
    if let Some(entry) = parsed_flags.iter_mut().find(|(n, _)| n == flag_name) {
        if multi {
            if let RuntimeValue::List(ref mut items) = entry.1 {
                items.push(val);
            }
        } else {
            entry.1 = val;
        }
    }
}

/// Apply env fallback for flags not set via argv. Checks std::env::var.
fn apply_env_fallback(
    flags: &[FlagSpec],
    parsed_flags: &mut [(String, RuntimeValue)],
) -> Result<(), String> {
    for fspec in flags {
        if let Some(ref env_name) = fspec.env {
            let (_, current) = parsed_flags.iter().find(|(n, _)| *n == fspec.name).unwrap();
            // Only fallback if the flag is still at its default (nil for value flags, false for bool, [] for multi)
            let is_default = if fspec.multi {
                matches!(current, RuntimeValue::List(items) if items.is_empty())
            } else if fspec.flag_type == FlagType::Boolean {
                *current == RuntimeValue::Bool(false)
            } else {
                *current == RuntimeValue::Nil
            };
            if is_default {
                if let Ok(env_val) = std::env::var(env_name) {
                    let val = parse_env_value(fspec, &env_val)?;
                    if fspec.multi {
                        set_flag_value(parsed_flags, &fspec.name, val, true);
                    } else {
                        set_flag_value(parsed_flags, &fspec.name, val, false);
                    }
                }
            }
        }
    }
    Ok(())
}

/// Parse argv against a spec. Returns {:ok, result}, {:help, text}, {:version, text}, or {:error, msg}.
fn do_parse(spec: &CliSpec, argv: &[String]) -> RuntimeValue {
    let mut flag_values: Vec<(RuntimeValue, RuntimeValue)> = Vec::new();
    let mut positional: Vec<String> = Vec::new();
    let mut output_json = false;
    let mut i = 0;

    let mut parsed_flags = init_parsed_flags(&spec.flags);

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
                        set_flag_value(
                            &mut parsed_flags,
                            stripped,
                            RuntimeValue::Bool(false),
                            false,
                        );
                        i += 1;
                        continue;
                    }
                }
            }

            if let Some(fspec) = spec.flags.iter().find(|f| f.name == flag_name) {
                match &fspec.flag_type {
                    FlagType::Boolean => {
                        set_flag_value(
                            &mut parsed_flags,
                            flag_name,
                            RuntimeValue::Bool(true),
                            false,
                        );
                    }
                    _ => {
                        i += 1;
                        if i >= argv.len() {
                            return error_tuple(format!("flag --{flag_name} requires a value"));
                        }
                        match parse_flag_value(fspec, &argv[i]) {
                            Ok(val) => {
                                set_flag_value(&mut parsed_flags, flag_name, val, fspec.multi);
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
                let is_multi = fspec.multi;
                match &fspec.flag_type {
                    FlagType::Boolean => {
                        set_flag_value(
                            &mut parsed_flags,
                            &flag_name,
                            RuntimeValue::Bool(true),
                            false,
                        );
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
                                set_flag_value(&mut parsed_flags, &flag_name, val, is_multi);
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

        // Check if this is a subcommand (first positional, before any positional args consumed)
        if !spec.commands.is_empty() && positional.is_empty() {
            if let Some(cmd) = spec.commands.iter().find(|c| c.name == *arg) {
                // Dispatch remaining argv to subcommand parser
                return do_parse_command(spec, cmd, &argv[i + 1..]);
            }
        }

        // Positional argument
        positional.push(arg.clone());
        i += 1;
    }

    // If spec has commands and we didn't dispatch to one, report error
    if !spec.commands.is_empty() {
        if positional.is_empty() {
            let cmd_names: Vec<&str> = spec.commands.iter().map(|c| c.name.as_str()).collect();
            return error_tuple(format!(
                "missing command. Available commands: {}",
                cmd_names.join(", ")
            ));
        } else {
            let cmd_names: Vec<&str> = spec.commands.iter().map(|c| c.name.as_str()).collect();
            return error_tuple(format!(
                "unknown command '{}'. Available commands: {}",
                positional[0],
                cmd_names.join(", ")
            ));
        }
    }

    // Apply env fallback for flags not explicitly set
    if let Err(msg) = apply_env_fallback(&spec.flags, &mut parsed_flags) {
        return error_tuple(msg);
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
    let val = match &fspec.flag_type {
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
    }?;
    // Validate choices
    validate_choices(&fspec.name, raw, &fspec.choices)?;
    Ok(val)
}

/// Convert a raw env string to a RuntimeValue using the flag's type, then validate choices.
fn parse_env_value(fspec: &FlagSpec, raw: &str) -> Result<RuntimeValue, String> {
    parse_flag_value(fspec, raw)
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

    // --- Subcommand helpers ---

    fn make_cmd_spec() -> RuntimeValue {
        kw(vec![
            ("name", s("git")),
            ("version", s("2.0.0")),
            ("description", s("A version control tool")),
            (
                "commands",
                kw(vec![
                    (
                        "clone",
                        kw(vec![
                            ("description", s("Clone a repository")),
                            (
                                "flags",
                                kw(vec![
                                    (
                                        "depth",
                                        kw(vec![
                                            ("type", atom("integer")),
                                            ("doc", s("Shallow clone depth")),
                                        ]),
                                    ),
                                    (
                                        "bare",
                                        kw(vec![
                                            ("type", atom("boolean")),
                                            ("doc", s("Create a bare repository")),
                                        ]),
                                    ),
                                ]),
                            ),
                            (
                                "args",
                                kw(vec![(
                                    "url",
                                    kw(vec![
                                        ("doc", s("Repository URL")),
                                        ("required", RuntimeValue::Bool(true)),
                                    ]),
                                )]),
                            ),
                        ]),
                    ),
                    (
                        "commit",
                        kw(vec![
                            ("description", s("Record changes")),
                            (
                                "flags",
                                kw(vec![
                                    (
                                        "message",
                                        kw(vec![
                                            ("type", atom("string")),
                                            ("short", s("m")),
                                            ("required", RuntimeValue::Bool(true)),
                                            ("doc", s("Commit message")),
                                        ]),
                                    ),
                                    (
                                        "all",
                                        kw(vec![
                                            ("type", atom("boolean")),
                                            ("short", s("a")),
                                            ("doc", s("Stage all changes")),
                                        ]),
                                    ),
                                ]),
                            ),
                        ]),
                    ),
                    (
                        "status",
                        kw(vec![("description", s("Show working tree status"))]),
                    ),
                ]),
            ),
        ])
    }

    fn extract_error_msg(result: &RuntimeValue) -> String {
        match result {
            RuntimeValue::Tuple(tag, val) => {
                assert_eq!(**tag, atom("error"));
                match val.as_ref() {
                    RuntimeValue::String(msg) => msg.clone(),
                    _ => panic!("expected error message string"),
                }
            }
            _ => panic!("expected error tuple"),
        }
    }

    // --- Subcommand tests ---

    #[test]
    fn cli_module_subcommand_parse_with_flags_and_args() {
        let spec = make_cmd_spec();
        let result = host_cli_parse(&[
            spec,
            argv(&["clone", "--depth", "1", "https://example.com"]),
        ])
        .unwrap();
        let data = extract_ok(&result);
        assert_eq!(*get_map_field(data, "command"), s("clone"));
        let flags = get_map_field(data, "flags");
        assert_eq!(*get_map_field(flags, "depth"), int(1));
        assert_eq!(*get_map_field(flags, "bare"), RuntimeValue::Bool(false));
        let args_map = get_map_field(data, "args");
        assert_eq!(*get_map_field(args_map, "url"), s("https://example.com"));
    }

    #[test]
    fn cli_module_subcommand_help_shows_command_details() {
        let spec = make_cmd_spec();
        let result = host_cli_parse(&[spec, argv(&["clone", "--help"])]).unwrap();
        match &result {
            RuntimeValue::Tuple(tag, val) => {
                assert_eq!(**tag, atom("help"));
                if let RuntimeValue::String(text) = val.as_ref() {
                    assert!(text.contains("git clone"));
                    assert!(text.contains("Clone a repository"));
                    assert!(text.contains("--depth"));
                    assert!(text.contains("<url>"));
                    assert!(text.contains("--output-json"));
                } else {
                    panic!("expected help text string");
                }
            }
            _ => panic!("expected help tuple"),
        }
    }

    #[test]
    fn cli_module_root_help_lists_commands() {
        let spec = make_cmd_spec();
        let result = host_cli_parse(&[spec, argv(&["--help"])]).unwrap();
        match &result {
            RuntimeValue::Tuple(tag, val) => {
                assert_eq!(**tag, atom("help"));
                if let RuntimeValue::String(text) = val.as_ref() {
                    assert!(text.contains("COMMANDS:"));
                    assert!(text.contains("clone"));
                    assert!(text.contains("commit"));
                    assert!(text.contains("status"));
                    assert!(text.contains("Clone a repository"));
                } else {
                    panic!("expected help text string");
                }
            }
            _ => panic!("expected help tuple"),
        }
    }

    #[test]
    fn cli_module_subcommand_unknown_command_error() {
        let spec = make_cmd_spec();
        let result = host_cli_parse(&[spec, argv(&["push"])]).unwrap();
        let msg = extract_error_msg(&result);
        assert!(msg.contains("unknown command 'push'"));
        assert!(msg.contains("clone"));
        assert!(msg.contains("commit"));
        assert!(msg.contains("status"));
    }

    #[test]
    fn cli_module_subcommand_missing_command_error() {
        let spec = make_cmd_spec();
        let result = host_cli_parse(&[spec, argv(&[])]).unwrap();
        let msg = extract_error_msg(&result);
        assert!(msg.contains("missing command"));
        assert!(msg.contains("clone"));
    }

    #[test]
    fn cli_module_subcommand_positional_args() {
        let spec = make_cmd_spec();
        let result = host_cli_parse(&[spec, argv(&["clone", "https://gh.com/repo"])]).unwrap();
        let data = extract_ok(&result);
        let args_map = get_map_field(data, "args");
        assert_eq!(*get_map_field(args_map, "url"), s("https://gh.com/repo"));
    }

    #[test]
    fn cli_module_subcommand_boolean_and_string_flags() {
        let spec = make_cmd_spec();
        let result = host_cli_parse(&[spec, argv(&["commit", "-a", "-m", "fix bug"])]).unwrap();
        let data = extract_ok(&result);
        assert_eq!(*get_map_field(data, "command"), s("commit"));
        let flags = get_map_field(data, "flags");
        assert_eq!(*get_map_field(flags, "all"), RuntimeValue::Bool(true));
        assert_eq!(*get_map_field(flags, "message"), s("fix bug"));
    }

    #[test]
    fn cli_module_subcommand_output_json_inherited() {
        let spec = make_cmd_spec();
        let result = host_cli_parse(&[spec, argv(&["status", "--output-json"])]).unwrap();
        let data = extract_ok(&result);
        assert_eq!(*get_map_field(data, "command"), s("status"));
        assert_eq!(
            *get_map_field(data, "output_json"),
            RuntimeValue::Bool(true)
        );
    }

    #[test]
    fn cli_module_subcommand_result_has_command_field() {
        let spec = make_cmd_spec();
        let result = host_cli_parse(&[spec, argv(&["status"])]).unwrap();
        let data = extract_ok(&result);
        assert_eq!(*get_map_field(data, "command"), s("status"));
        // status has no flags or args, so maps should be empty
        let flags = get_map_field(data, "flags");
        assert_eq!(*flags, RuntimeValue::Map(vec![]));
        let args_map = get_map_field(data, "args");
        assert_eq!(*args_map, RuntimeValue::Map(vec![]));
    }

    #[test]
    fn cli_module_subcommand_bare_command_no_flags() {
        let spec = make_cmd_spec();
        let result = host_cli_parse(&[spec, argv(&["status"])]).unwrap();
        let data = extract_ok(&result);
        assert_eq!(*get_map_field(data, "command"), s("status"));
        assert_eq!(
            *get_map_field(data, "output_json"),
            RuntimeValue::Bool(false)
        );
    }

    #[test]
    fn cli_module_subcommand_required_flag_missing() {
        let spec = make_cmd_spec();
        let result = host_cli_parse(&[spec, argv(&["commit"])]).unwrap();
        let msg = extract_error_msg(&result);
        assert!(msg.contains("message"));
        assert!(msg.contains("missing"));
    }

    #[test]
    fn cli_module_subcommand_multiple_commands_in_spec() {
        // Verify all three commands are parseable
        let spec1 = make_cmd_spec();
        let r1 = host_cli_parse(&[spec1, argv(&["clone", "url"])]).unwrap();
        assert_eq!(*get_map_field(extract_ok(&r1), "command"), s("clone"));

        let spec2 = make_cmd_spec();
        let r2 = host_cli_parse(&[spec2, argv(&["commit", "-m", "msg"])]).unwrap();
        assert_eq!(*get_map_field(extract_ok(&r2), "command"), s("commit"));

        let spec3 = make_cmd_spec();
        let r3 = host_cli_parse(&[spec3, argv(&["status"])]).unwrap();
        assert_eq!(*get_map_field(extract_ok(&r3), "command"), s("status"));
    }

    // --- Choices tests ---

    fn make_choices_spec() -> RuntimeValue {
        kw(vec![
            ("name", s("app")),
            (
                "flags",
                kw(vec![(
                    "format",
                    kw(vec![
                        ("type", atom("string")),
                        ("short", s("f")),
                        ("doc", s("Output format")),
                        ("choices", list(vec![s("json"), s("csv"), s("text")])),
                        ("default", s("json")),
                    ]),
                )]),
            ),
        ])
    }

    #[test]
    fn cli_module_choices_valid_value_accepted() {
        let spec = make_choices_spec();
        let result = host_cli_parse(&[spec, argv(&["--format", "csv"])]).unwrap();
        let data = extract_ok(&result);
        let flags = get_map_field(data, "flags");
        assert_eq!(*get_map_field(flags, "format"), s("csv"));
    }

    #[test]
    fn cli_module_choices_invalid_value_rejected() {
        let spec = make_choices_spec();
        let result = host_cli_parse(&[spec, argv(&["--format", "xml"])]).unwrap();
        let msg = extract_error_msg(&result);
        assert!(msg.contains("invalid value 'xml'"));
        assert!(msg.contains("--format"));
        assert!(msg.contains("json"));
        assert!(msg.contains("csv"));
        assert!(msg.contains("text"));
    }

    #[test]
    fn cli_module_choices_shown_in_help() {
        let spec = make_choices_spec();
        let result = host_cli_parse(&[spec, argv(&["--help"])]).unwrap();
        match &result {
            RuntimeValue::Tuple(_, val) => {
                if let RuntimeValue::String(text) = val.as_ref() {
                    assert!(text.contains("json, csv, text"));
                } else {
                    panic!("expected help text");
                }
            }
            _ => panic!("expected help tuple"),
        }
    }

    #[test]
    fn cli_module_choices_with_integer_type() {
        let spec = kw(vec![
            ("name", s("app")),
            (
                "flags",
                kw(vec![(
                    "level",
                    kw(vec![
                        ("type", atom("integer")),
                        ("choices", list(vec![s("1"), s("2"), s("3")])),
                    ]),
                )]),
            ),
        ]);
        let result = host_cli_parse(&[spec.clone(), argv(&["--level", "2"])]).unwrap();
        let data = extract_ok(&result);
        let flags = get_map_field(data, "flags");
        assert_eq!(*get_map_field(flags, "level"), int(2));

        let spec2 = spec;
        let result2 = host_cli_parse(&[spec2, argv(&["--level", "5"])]).unwrap();
        let msg = extract_error_msg(&result2);
        assert!(msg.contains("invalid value '5'"));
    }

    // --- Env fallback tests ---

    fn make_env_spec() -> RuntimeValue {
        kw(vec![
            ("name", s("app")),
            (
                "flags",
                kw(vec![(
                    "port",
                    kw(vec![
                        ("type", atom("integer")),
                        ("doc", s("Server port")),
                        ("env", s("TEST_CLI_PORT")),
                        ("default", int(8080)),
                    ]),
                )]),
            ),
        ])
    }

    #[test]
    fn cli_module_env_fallback_reads_from_env() {
        std::env::set_var("TEST_CLI_PORT_READ", "9090");
        let spec = kw(vec![
            ("name", s("app")),
            (
                "flags",
                kw(vec![(
                    "port",
                    kw(vec![
                        ("type", atom("integer")),
                        ("env", s("TEST_CLI_PORT_READ")),
                    ]),
                )]),
            ),
        ]);
        let result = host_cli_parse(&[spec, argv(&[])]).unwrap();
        let data = extract_ok(&result);
        let flags = get_map_field(data, "flags");
        assert_eq!(*get_map_field(flags, "port"), int(9090));
        std::env::remove_var("TEST_CLI_PORT_READ");
    }

    #[test]
    fn cli_module_env_argv_takes_precedence() {
        std::env::set_var("TEST_CLI_PORT_PREC", "9090");
        let spec = kw(vec![
            ("name", s("app")),
            (
                "flags",
                kw(vec![(
                    "port",
                    kw(vec![
                        ("type", atom("integer")),
                        ("env", s("TEST_CLI_PORT_PREC")),
                    ]),
                )]),
            ),
        ]);
        let result = host_cli_parse(&[spec, argv(&["--port", "3000"])]).unwrap();
        let data = extract_ok(&result);
        let flags = get_map_field(data, "flags");
        assert_eq!(*get_map_field(flags, "port"), int(3000));
        std::env::remove_var("TEST_CLI_PORT_PREC");
    }

    #[test]
    fn cli_module_env_shown_in_help() {
        let spec = make_env_spec();
        let result = host_cli_parse(&[spec, argv(&["--help"])]).unwrap();
        match &result {
            RuntimeValue::Tuple(_, val) => {
                if let RuntimeValue::String(text) = val.as_ref() {
                    assert!(text.contains("env: TEST_CLI_PORT"));
                } else {
                    panic!("expected help text");
                }
            }
            _ => panic!("expected help tuple"),
        }
    }

    #[test]
    fn cli_module_env_type_coerced() {
        std::env::set_var("TEST_CLI_PORT_COERCE", "4567");
        let spec = kw(vec![
            ("name", s("app")),
            (
                "flags",
                kw(vec![(
                    "port",
                    kw(vec![
                        ("type", atom("integer")),
                        ("env", s("TEST_CLI_PORT_COERCE")),
                    ]),
                )]),
            ),
        ]);
        let result = host_cli_parse(&[spec, argv(&[])]).unwrap();
        let data = extract_ok(&result);
        let flags = get_map_field(data, "flags");
        assert_eq!(*get_map_field(flags, "port"), int(4567));
        std::env::remove_var("TEST_CLI_PORT_COERCE");
    }

    #[test]
    fn cli_module_env_validated_against_choices() {
        std::env::set_var("TEST_CLI_FMT_CHOICES", "xml");
        let spec = kw(vec![
            ("name", s("app")),
            (
                "flags",
                kw(vec![(
                    "format",
                    kw(vec![
                        ("type", atom("string")),
                        ("env", s("TEST_CLI_FMT_CHOICES")),
                        ("choices", list(vec![s("json"), s("csv")])),
                    ]),
                )]),
            ),
        ]);
        let result = host_cli_parse(&[spec, argv(&[])]).unwrap();
        let msg = extract_error_msg(&result);
        assert!(msg.contains("invalid value 'xml'"));
        assert!(msg.contains("--format"));
        std::env::remove_var("TEST_CLI_FMT_CHOICES");
    }

    // --- Multi-value flag tests ---

    fn make_multi_spec() -> RuntimeValue {
        kw(vec![
            ("name", s("app")),
            (
                "flags",
                kw(vec![(
                    "tag",
                    kw(vec![
                        ("type", atom("string")),
                        ("short", s("t")),
                        ("doc", s("Tags")),
                        ("multi", RuntimeValue::Bool(true)),
                    ]),
                )]),
            ),
        ])
    }

    #[test]
    fn cli_module_multi_repeated_flags_collected() {
        let spec = make_multi_spec();
        let result = host_cli_parse(&[
            spec,
            argv(&["--tag", "foo", "--tag", "bar", "--tag", "baz"]),
        ])
        .unwrap();
        let data = extract_ok(&result);
        let flags = get_map_field(data, "flags");
        assert_eq!(
            *get_map_field(flags, "tag"),
            list(vec![s("foo"), s("bar"), s("baz")])
        );
    }

    #[test]
    fn cli_module_multi_single_value_becomes_list() {
        let spec = make_multi_spec();
        let result = host_cli_parse(&[spec, argv(&["--tag", "only"])]).unwrap();
        let data = extract_ok(&result);
        let flags = get_map_field(data, "flags");
        assert_eq!(*get_map_field(flags, "tag"), list(vec![s("only")]));
    }

    #[test]
    fn cli_module_multi_no_values_gives_empty_list() {
        let spec = make_multi_spec();
        let result = host_cli_parse(&[spec, argv(&[])]).unwrap();
        let data = extract_ok(&result);
        let flags = get_map_field(data, "flags");
        assert_eq!(*get_map_field(flags, "tag"), list(vec![]));
    }

    #[test]
    fn cli_module_multi_each_value_type_checked() {
        let spec = kw(vec![
            ("name", s("app")),
            (
                "flags",
                kw(vec![(
                    "count",
                    kw(vec![
                        ("type", atom("integer")),
                        ("multi", RuntimeValue::Bool(true)),
                    ]),
                )]),
            ),
        ]);
        let result = host_cli_parse(&[spec, argv(&["--count", "1", "--count", "abc"])]).unwrap();
        let msg = extract_error_msg(&result);
        assert!(msg.contains("integer"));
        assert!(msg.contains("abc"));
    }

    #[test]
    fn cli_module_multi_shown_in_help() {
        let spec = make_multi_spec();
        let result = host_cli_parse(&[spec, argv(&["--help"])]).unwrap();
        match &result {
            RuntimeValue::Tuple(_, val) => {
                if let RuntimeValue::String(text) = val.as_ref() {
                    assert!(text.contains("can be repeated"));
                } else {
                    panic!("expected help text");
                }
            }
            _ => panic!("expected help tuple"),
        }
    }

    // --- Combined tests ---

    #[test]
    fn cli_module_multi_plus_choices_validation() {
        let spec = kw(vec![
            ("name", s("app")),
            (
                "flags",
                kw(vec![(
                    "format",
                    kw(vec![
                        ("type", atom("string")),
                        ("multi", RuntimeValue::Bool(true)),
                        ("choices", list(vec![s("json"), s("csv")])),
                    ]),
                )]),
            ),
        ]);
        // Valid values
        let result =
            host_cli_parse(&[spec.clone(), argv(&["--format", "json", "--format", "csv"])])
                .unwrap();
        let data = extract_ok(&result);
        let flags = get_map_field(data, "flags");
        assert_eq!(
            *get_map_field(flags, "format"),
            list(vec![s("json"), s("csv")])
        );

        // Invalid value in multi
        let result2 =
            host_cli_parse(&[spec, argv(&["--format", "json", "--format", "xml"])]).unwrap();
        let msg = extract_error_msg(&result2);
        assert!(msg.contains("invalid value 'xml'"));
    }

    #[test]
    fn cli_module_env_plus_choices_validation() {
        std::env::set_var("TEST_CLI_COMBO_FMT", "csv");
        let spec = kw(vec![
            ("name", s("app")),
            (
                "flags",
                kw(vec![(
                    "format",
                    kw(vec![
                        ("type", atom("string")),
                        ("env", s("TEST_CLI_COMBO_FMT")),
                        ("choices", list(vec![s("json"), s("csv")])),
                    ]),
                )]),
            ),
        ]);
        let result = host_cli_parse(&[spec, argv(&[])]).unwrap();
        let data = extract_ok(&result);
        let flags = get_map_field(data, "flags");
        assert_eq!(*get_map_field(flags, "format"), s("csv"));
        std::env::remove_var("TEST_CLI_COMBO_FMT");
    }
}
