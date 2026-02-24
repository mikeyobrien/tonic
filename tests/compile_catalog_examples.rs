use serde::Deserialize;
use std::fs;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct Catalog {
    example: Vec<CatalogEntry>,
}

#[derive(Debug, Deserialize)]
struct CatalogEntry {
    path: String,
    check_exit: i32,
    status: String,
    blocked_reason: Option<String>,
}

#[test]
fn compile_matches_catalog_expectations_for_active_examples() {
    let catalog_str = fs::read_to_string("examples/parity/catalog.toml")
        .expect("Failed to read examples/parity/catalog.toml");
    let catalog: Catalog = toml::from_str(&catalog_str).expect("Failed to parse catalog.toml");

    for entry in catalog.example {
        if entry.status == "blocked" {
            println!(
                "SKIPPED {}: blocked - {}",
                entry.path,
                entry
                    .blocked_reason
                    .as_deref()
                    .unwrap_or("no reason provided")
            );
            continue;
        }

        assert_eq!(entry.status, "active", "Status must be active or blocked");

        let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
            .args(["compile", &entry.path])
            .output()
            .unwrap_or_else(|_| panic!("Failed to execute tonic compile for {}", entry.path));

        let compile_exit = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert_eq!(
            compile_exit, entry.check_exit,
            "[{}] compile exit code mismatch. expected {}, got {}\nstdout: {}\nstderr: {}",
            entry.path, entry.check_exit, compile_exit, stdout, stderr
        );

        if compile_exit == 0 {
            assert!(
                stdout.contains("compile: ok"),
                "[{}] expected successful compile marker in stdout, got {:?}",
                entry.path,
                stdout
            );
        } else {
            assert!(
                stderr.contains("error:"),
                "[{}] expected compile diagnostics in stderr, got {:?}",
                entry.path,
                stderr
            );
        }
    }
}
