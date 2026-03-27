use super::{HostError, HostRegistry};
use crate::runtime::RuntimeValue;

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
    match &args[index] {
        RuntimeValue::String(s) => Ok(s.clone()),
        other => Err(HostError::new(format!(
            "{} expects a string argument at position {}, got {:?}",
            function,
            index + 1,
            std::mem::discriminant(other)
        ))),
    }
}

/// Copy a file from source to destination.
fn file_cp(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("File.cp", args, 2)?;
    let src = expect_string_arg("File.cp", args, 0)?;
    let dst = expect_string_arg("File.cp", args, 1)?;
    match std::fs::copy(&src, &dst) {
        Ok(_) => Ok(RuntimeValue::Atom("ok".to_string())),
        Err(e) => Ok(RuntimeValue::Tuple(
            Box::new(RuntimeValue::Atom("error".to_string())),
            Box::new(RuntimeValue::String(e.to_string())),
        )),
    }
}

/// Rename (move) a file or directory.
fn file_rename(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("File.rename", args, 2)?;
    let src = expect_string_arg("File.rename", args, 0)?;
    let dst = expect_string_arg("File.rename", args, 1)?;
    match std::fs::rename(&src, &dst) {
        Ok(()) => Ok(RuntimeValue::Atom("ok".to_string())),
        Err(e) => Ok(RuntimeValue::Tuple(
            Box::new(RuntimeValue::Atom("error".to_string())),
            Box::new(RuntimeValue::String(e.to_string())),
        )),
    }
}

/// Return file metadata as a map: %{size: int, is_dir: bool, is_file: bool}.
fn file_stat(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("File.stat", args, 1)?;
    let path = expect_string_arg("File.stat", args, 0)?;
    match std::fs::metadata(&path) {
        Ok(meta) => {
            let entries = vec![
                (
                    RuntimeValue::String("size".to_string()),
                    RuntimeValue::Int(meta.len() as i64),
                ),
                (
                    RuntimeValue::String("is_dir".to_string()),
                    RuntimeValue::Bool(meta.is_dir()),
                ),
                (
                    RuntimeValue::String("is_file".to_string()),
                    RuntimeValue::Bool(meta.is_file()),
                ),
            ];
            Ok(RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("ok".to_string())),
                Box::new(RuntimeValue::Map(entries)),
            ))
        }
        Err(e) => Ok(RuntimeValue::Tuple(
            Box::new(RuntimeValue::Atom("error".to_string())),
            Box::new(RuntimeValue::String(e.to_string())),
        )),
    }
}

pub(crate) fn register_file_host_functions(registry: &HostRegistry) {
    registry.register("file_cp", file_cp);
    registry.register("file_rename", file_rename);
    registry.register("file_stat", file_stat);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cp_copies_file_contents() {
        let dir = std::env::temp_dir().join("tonic_file_cp_test");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("source.txt");
        let dst = dir.join("dest.txt");
        std::fs::write(&src, "hello file").unwrap();
        let _ = std::fs::remove_file(&dst);

        let result = file_cp(&[
            RuntimeValue::String(src.to_string_lossy().to_string()),
            RuntimeValue::String(dst.to_string_lossy().to_string()),
        ]);
        assert_eq!(result.unwrap(), RuntimeValue::Atom("ok".to_string()));
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "hello file");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cp_returns_error_for_missing_source() {
        let result = file_cp(&[
            RuntimeValue::String("/tmp/tonic_nonexistent_file_cp_test".to_string()),
            RuntimeValue::String("/tmp/tonic_cp_dst".to_string()),
        ]);
        match result.unwrap() {
            RuntimeValue::Tuple(first, second) => {
                assert_eq!(*first, RuntimeValue::Atom("error".to_string()));
                if let RuntimeValue::String(msg) = &*second {
                    assert!(msg.contains("No such file") || msg.contains("not found"));
                } else {
                    panic!("expected string error message");
                }
            }
            other => panic!("expected error tuple, got {:?}", other),
        }
    }

    #[test]
    fn rename_moves_file() {
        let dir = std::env::temp_dir().join("tonic_file_rename_test");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("before.txt");
        let dst = dir.join("after.txt");
        std::fs::write(&src, "rename me").unwrap();
        let _ = std::fs::remove_file(&dst);

        let result = file_rename(&[
            RuntimeValue::String(src.to_string_lossy().to_string()),
            RuntimeValue::String(dst.to_string_lossy().to_string()),
        ]);
        assert_eq!(result.unwrap(), RuntimeValue::Atom("ok".to_string()));
        assert!(!src.exists());
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "rename me");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rename_returns_error_for_missing_source() {
        let result = file_rename(&[
            RuntimeValue::String("/tmp/tonic_nonexistent_rename_test".to_string()),
            RuntimeValue::String("/tmp/tonic_rename_dst".to_string()),
        ]);
        match result.unwrap() {
            RuntimeValue::Tuple(first, _second) => {
                assert_eq!(*first, RuntimeValue::Atom("error".to_string()));
            }
            other => panic!("expected error tuple, got {:?}", other),
        }
    }

    #[test]
    fn stat_returns_file_metadata() {
        let dir = std::env::temp_dir().join("tonic_file_stat_test");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("test.txt");
        std::fs::write(&file, "12345").unwrap();

        let result = file_stat(&[RuntimeValue::String(file.to_string_lossy().to_string())]);
        match result.unwrap() {
            RuntimeValue::Tuple(first, second) => {
                assert_eq!(*first, RuntimeValue::Atom("ok".to_string()));
                if let RuntimeValue::Map(entries) = &*second {
                    let size = entries
                        .iter()
                        .find(|(k, _)| k == &RuntimeValue::String("size".to_string()))
                        .map(|(_, v)| v.clone());
                    assert_eq!(size, Some(RuntimeValue::Int(5)));

                    let is_file = entries
                        .iter()
                        .find(|(k, _)| k == &RuntimeValue::String("is_file".to_string()))
                        .map(|(_, v)| v.clone());
                    assert_eq!(is_file, Some(RuntimeValue::Bool(true)));

                    let is_dir = entries
                        .iter()
                        .find(|(k, _)| k == &RuntimeValue::String("is_dir".to_string()))
                        .map(|(_, v)| v.clone());
                    assert_eq!(is_dir, Some(RuntimeValue::Bool(false)));
                } else {
                    panic!("expected map in stat result");
                }
            }
            other => panic!("expected ok tuple, got {:?}", other),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stat_returns_dir_metadata() {
        let dir = std::env::temp_dir().join("tonic_file_stat_dir_test");
        let _ = std::fs::create_dir_all(&dir);

        let result = file_stat(&[RuntimeValue::String(dir.to_string_lossy().to_string())]);
        match result.unwrap() {
            RuntimeValue::Tuple(first, second) => {
                assert_eq!(*first, RuntimeValue::Atom("ok".to_string()));
                if let RuntimeValue::Map(entries) = &*second {
                    let is_dir = entries
                        .iter()
                        .find(|(k, _)| k == &RuntimeValue::String("is_dir".to_string()))
                        .map(|(_, v)| v.clone());
                    assert_eq!(is_dir, Some(RuntimeValue::Bool(true)));
                }
            }
            _ => panic!("expected ok tuple"),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stat_returns_error_for_missing_path() {
        let result = file_stat(&[RuntimeValue::String(
            "/tmp/tonic_nonexistent_stat_test".to_string(),
        )]);
        match result.unwrap() {
            RuntimeValue::Tuple(first, _second) => {
                assert_eq!(*first, RuntimeValue::Atom("error".to_string()));
            }
            other => panic!("expected error tuple, got {:?}", other),
        }
    }

    #[test]
    fn cp_rejects_wrong_arity() {
        let result = file_cp(&[RuntimeValue::String("one".to_string())]);
        assert!(result.is_err());
    }

    #[test]
    fn rename_rejects_wrong_arity() {
        let result = file_rename(&[RuntimeValue::String("one".to_string())]);
        assert!(result.is_err());
    }

    #[test]
    fn stat_rejects_wrong_arity() {
        let result = file_stat(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn stat_rejects_non_string() {
        let result = file_stat(&[RuntimeValue::Int(42)]);
        assert!(result.is_err());
    }

    #[test]
    fn register_adds_all_functions() {
        let registry = HostRegistry::new();
        // file_mod is registered via register_sample_functions → new()
        // Verify by calling with wrong arity (proves function is registered)
        assert!(registry
            .call("file_cp", &[RuntimeValue::String("a".to_string())])
            .is_err());
        assert!(registry
            .call("file_rename", &[RuntimeValue::String("a".to_string())])
            .is_err());
        assert!(registry.call("file_stat", &[]).is_err());
    }
}
