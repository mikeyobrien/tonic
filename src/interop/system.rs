use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;
use std::path::{Path, PathBuf};

fn expect_exact_args(
    function: &str,
    args: &[RuntimeValue],
    expected: usize,
) -> Result<(), HostError> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(HostError::new(format!(
            "{} expects exactly {} argument{}, found {}",
            function,
            expected,
            if expected == 1 { "" } else { "s" },
            args.len()
        )))
    }
}

fn expect_string_arg(
    function: &str,
    args: &[RuntimeValue],
    index: usize,
) -> Result<String, HostError> {
    let Some(value) = args.get(index) else {
        return Err(HostError::new(format!(
            "{} missing required argument {}",
            function,
            index + 1
        )));
    };

    match value {
        RuntimeValue::String(text) => Ok(text.clone()),
        other => Err(HostError::new(format!(
            "{} expects string argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

fn atom_key(key: &str) -> RuntimeValue {
    RuntimeValue::Atom(key.to_string())
}

fn map_with_atom_keys(entries: Vec<(&str, RuntimeValue)>) -> RuntimeValue {
    RuntimeValue::Map(
        entries
            .into_iter()
            .map(|(key, value)| (atom_key(key), value))
            .collect(),
    )
}

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn find_command_on_path(name: &str) -> Option<PathBuf> {
    let candidate = Path::new(name);
    if candidate.components().count() > 1 {
        return is_executable_file(candidate).then(|| candidate.to_path_buf());
    }

    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let direct = dir.join(name);
        if is_executable_file(&direct) {
            return Some(direct);
        }

        if cfg!(windows) {
            for ext in ["exe", "cmd", "bat"] {
                let with_ext = dir.join(format!("{name}.{ext}"));
                if is_executable_file(&with_ext) {
                    return Some(with_ext);
                }
            }
        }
    }

    None
}

fn host_sys_run(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_run", args, 1)?;
    let command = expect_string_arg("sys_run", args, 0)?;
    let shell_command = format!("{command} 2>&1");

    let output = std::process::Command::new("sh")
        .args(["-lc", &shell_command])
        .output()
        .map_err(|error| {
            HostError::new(format!("sys_run failed to execute shell command: {error}"))
        })?;

    let exit_code = output.status.code().unwrap_or(-1);
    let combined_output = String::from_utf8_lossy(&output.stdout).into_owned();

    Ok(map_with_atom_keys(vec![
        ("exit_code", RuntimeValue::Int(exit_code as i64)),
        ("output", RuntimeValue::String(combined_output)),
    ]))
}

fn host_sys_path_exists(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_path_exists", args, 1)?;
    let path = expect_string_arg("sys_path_exists", args, 0)?;
    Ok(RuntimeValue::Bool(Path::new(&path).exists()))
}

fn host_sys_ensure_dir(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_ensure_dir", args, 1)?;
    let path = expect_string_arg("sys_ensure_dir", args, 0)?;

    std::fs::create_dir_all(&path).map_err(|error| {
        HostError::new(format!("sys_ensure_dir failed for '{}': {error}", path))
    })?;

    Ok(RuntimeValue::Bool(true))
}

fn host_sys_write_text(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_write_text", args, 2)?;
    let path = expect_string_arg("sys_write_text", args, 0)?;
    let content = expect_string_arg("sys_write_text", args, 1)?;

    std::fs::write(&path, content).map_err(|error| {
        HostError::new(format!("sys_write_text failed for '{}': {error}", path))
    })?;

    Ok(RuntimeValue::Bool(true))
}

fn host_sys_env(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_env", args, 1)?;
    let key = expect_string_arg("sys_env", args, 0)?;

    let value = std::env::var_os(&key)
        .map(|v| RuntimeValue::String(v.to_string_lossy().into_owned()))
        .unwrap_or(RuntimeValue::Nil);

    Ok(value)
}

fn host_sys_which(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_which", args, 1)?;
    let command = expect_string_arg("sys_which", args, 0)?;

    let value = find_command_on_path(&command)
        .map(|path| RuntimeValue::String(path.display().to_string()))
        .unwrap_or(RuntimeValue::Nil);

    Ok(value)
}

fn host_sys_cwd(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_cwd", args, 0)?;

    let cwd = std::env::current_dir()
        .map_err(|error| HostError::new(format!("sys_cwd failed to read current dir: {error}")))?;

    Ok(RuntimeValue::String(cwd.display().to_string()))
}

fn host_sys_argv(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_argv", args, 0)?;

    let argv_list = std::env::args()
        .map(RuntimeValue::String)
        .collect::<Vec<_>>();

    Ok(RuntimeValue::List(argv_list))
}

pub(super) fn register_system_host_functions(registry: &HostRegistry) {
    registry.register("sys_run", host_sys_run);
    registry.register("sys_path_exists", host_sys_path_exists);
    registry.register("sys_ensure_dir", host_sys_ensure_dir);
    registry.register("sys_write_text", host_sys_write_text);
    registry.register("sys_env", host_sys_env);
    registry.register("sys_which", host_sys_which);
    registry.register("sys_cwd", host_sys_cwd);
    registry.register("sys_argv", host_sys_argv);
}
