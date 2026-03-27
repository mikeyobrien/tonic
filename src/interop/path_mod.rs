use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;
use std::path::Path;

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
        RuntimeValue::String(s) => Ok(s.clone()),
        other => Err(HostError::new(format!(
            "{} expects string argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

fn host_path_join(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Path.join", args, 2)?;
    let a = expect_string_arg("Path.join", args, 0)?;
    let b = expect_string_arg("Path.join", args, 1)?;
    let joined = Path::new(&a).join(&b);
    Ok(RuntimeValue::String(joined.to_string_lossy().into_owned()))
}

fn host_path_dirname(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Path.dirname", args, 1)?;
    let path = expect_string_arg("Path.dirname", args, 0)?;
    let dir = Path::new(&path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_string());
    Ok(RuntimeValue::String(dir))
}

fn host_path_basename(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Path.basename", args, 1)?;
    let path = expect_string_arg("Path.basename", args, 0)?;
    let base = Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(RuntimeValue::String(base))
}

fn host_path_extname(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Path.extname", args, 1)?;
    let path = expect_string_arg("Path.extname", args, 0)?;
    let ext = Path::new(&path)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    Ok(RuntimeValue::String(ext))
}

fn host_path_expand(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Path.expand", args, 1)?;
    let path = expect_string_arg("Path.expand", args, 0)?;

    // Expand ~ to home directory
    let expanded = if path.starts_with("~/") || path == "~" {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        if path == "~" {
            home
        } else {
            format!("{home}{}", &path[1..])
        }
    } else {
        path
    };

    let p = Path::new(&expanded);
    let absolute = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| HostError::new(format!("Path.expand could not get cwd: {e}")))?
            .join(p)
    };

    Ok(RuntimeValue::String(
        absolute.to_string_lossy().into_owned(),
    ))
}

fn host_path_relative_to(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Path.relative_to", args, 2)?;
    let path = expect_string_arg("Path.relative_to", args, 0)?;
    let base = expect_string_arg("Path.relative_to", args, 1)?;

    let p = Path::new(&path);
    let b = Path::new(&base);

    // Strip base prefix if possible
    match p.strip_prefix(b) {
        Ok(relative) => Ok(RuntimeValue::String(
            relative.to_string_lossy().into_owned(),
        )),
        // Return original path if base is not a prefix
        Err(_) => Ok(RuntimeValue::String(path)),
    }
}

fn host_path_rootname(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Path.rootname", args, 1)?;
    let path = expect_string_arg("Path.rootname", args, 0)?;
    let p = Path::new(&path);
    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned());
    let parent = p.parent();
    let result = match (parent, stem) {
        (Some(dir), Some(stem)) if dir == Path::new("") => stem,
        (Some(dir), Some(stem)) => format!("{}/{}", dir.to_string_lossy(), stem),
        _ => path,
    };
    Ok(RuntimeValue::String(result))
}

fn host_path_split(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Path.split", args, 1)?;
    let path = expect_string_arg("Path.split", args, 0)?;
    let components: Vec<RuntimeValue> = Path::new(&path)
        .components()
        .map(|c| RuntimeValue::String(c.as_os_str().to_string_lossy().into_owned()))
        .collect();
    Ok(RuntimeValue::List(components))
}

pub fn register_path_host_functions(registry: &HostRegistry) {
    registry.register("path_join", host_path_join);
    registry.register("path_dirname", host_path_dirname);
    registry.register("path_basename", host_path_basename);
    registry.register("path_extname", host_path_extname);
    registry.register("path_expand", host_path_expand);
    registry.register("path_relative_to", host_path_relative_to);
    registry.register("path_rootname", host_path_rootname);
    registry.register("path_split", host_path_split);
}

#[cfg(test)]
mod tests {
    use crate::interop::HOST_REGISTRY;
    use crate::runtime::RuntimeValue;

    fn s(text: &str) -> RuntimeValue {
        RuntimeValue::String(text.to_string())
    }

    #[test]
    fn path_join_joins_segments() {
        let result = HOST_REGISTRY
            .call("path_join", &[s("/tmp"), s("file.txt")])
            .expect("path_join should succeed");
        assert_eq!(result, s("/tmp/file.txt"));
    }

    #[test]
    fn path_dirname_returns_parent_directory() {
        let result = HOST_REGISTRY
            .call("path_dirname", &[s("/tmp/foo/bar.txt")])
            .expect("path_dirname should succeed");
        assert_eq!(result, s("/tmp/foo"));
    }

    #[test]
    fn path_basename_returns_filename() {
        let result = HOST_REGISTRY
            .call("path_basename", &[s("/tmp/foo/bar.txt")])
            .expect("path_basename should succeed");
        assert_eq!(result, s("bar.txt"));
    }

    #[test]
    fn path_extname_returns_extension() {
        let result = HOST_REGISTRY
            .call("path_extname", &[s("/tmp/foo/bar.txt")])
            .expect("path_extname should succeed");
        assert_eq!(result, s(".txt"));
    }

    #[test]
    fn path_extname_returns_empty_for_no_extension() {
        let result = HOST_REGISTRY
            .call("path_extname", &[s("/tmp/foo/noext")])
            .expect("path_extname should succeed for file without extension");
        assert_eq!(result, s(""));
    }

    #[test]
    fn path_relative_to_strips_base_prefix() {
        let result = HOST_REGISTRY
            .call("path_relative_to", &[s("/tmp/foo/bar.txt"), s("/tmp/foo")])
            .expect("path_relative_to should succeed");
        assert_eq!(result, s("bar.txt"));
    }

    #[test]
    fn path_relative_to_returns_original_when_no_prefix() {
        let result = HOST_REGISTRY
            .call("path_relative_to", &[s("/tmp/foo/bar.txt"), s("/other")])
            .expect("path_relative_to should succeed when base is not a prefix");
        assert_eq!(result, s("/tmp/foo/bar.txt"));
    }

    #[test]
    fn path_expand_converts_relative_to_absolute() {
        let result = HOST_REGISTRY
            .call("path_expand", &[s(".")])
            .expect("path_expand should succeed");
        match result {
            RuntimeValue::String(path) => assert!(
                path.starts_with('/'),
                "expanded path should be absolute, got: {path}"
            ),
            other => panic!("expected string from path_expand, got {:?}", other),
        }
    }

    #[test]
    fn path_rootname_strips_extension() {
        let result = HOST_REGISTRY
            .call("path_rootname", &[s("/tmp/foo/bar.txt")])
            .expect("path_rootname should succeed");
        assert_eq!(result, s("/tmp/foo/bar"));
    }

    #[test]
    fn path_rootname_no_extension() {
        let result = HOST_REGISTRY
            .call("path_rootname", &[s("/tmp/foo/bar")])
            .expect("path_rootname should succeed with no extension");
        assert_eq!(result, s("/tmp/foo/bar"));
    }

    #[test]
    fn path_rootname_bare_filename() {
        let result = HOST_REGISTRY
            .call("path_rootname", &[s("bar.txt")])
            .expect("path_rootname should work on bare filename");
        assert_eq!(result, s("bar"));
    }

    #[test]
    fn path_split_splits_into_components() {
        let result = HOST_REGISTRY
            .call("path_split", &[s("/tmp/foo/bar.txt")])
            .expect("path_split should succeed");
        assert_eq!(
            result,
            RuntimeValue::List(vec![s("/"), s("tmp"), s("foo"), s("bar.txt")])
        );
    }

    #[test]
    fn path_split_relative_path() {
        let result = HOST_REGISTRY
            .call("path_split", &[s("foo/bar/baz")])
            .expect("path_split should succeed on relative path");
        assert_eq!(
            result,
            RuntimeValue::List(vec![s("foo"), s("bar"), s("baz")])
        );
    }

    #[test]
    fn path_join_rejects_wrong_arity() {
        let error = HOST_REGISTRY
            .call("path_join", &[s("/tmp")])
            .expect_err("path_join should reject wrong arity");
        assert_eq!(
            error.to_string(),
            "host error: Path.join expects exactly 2 arguments, found 1"
        );
    }
}
