use serde::Deserialize;
use std::fs;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct Catalog {
    example: Vec<Example>,
}

#[derive(Debug, Deserialize)]
struct Example {
    path: String,
    check_exit: i32,
    run_exit: i32,
    stdout: Option<String>,
    stderr_contains: Option<String>,
    status: String,
    blocked_reason: Option<String>,
}

#[test]
fn test_parity_examples() {
    let catalog_str = fs::read_to_string("examples/parity/catalog.toml")
        .expect("Failed to read examples/parity/catalog.toml");
    let catalog: Catalog = toml::from_str(&catalog_str).expect("Failed to parse catalog.toml");

    for example in catalog.example {
        println!("Running example: {}", example.path);

        if example.status == "blocked" {
            println!(
                "SKIPPED {}: blocked - {}",
                example.path,
                example
                    .blocked_reason
                    .as_deref()
                    .unwrap_or("no reason provided")
            );
            continue;
        }

        assert_eq!(example.status, "active", "Status must be active or blocked");

        // 1. tonic check
        let check_output = Command::new(env!("CARGO_BIN_EXE_tonic"))
            .args(["check", &example.path])
            .output()
            .unwrap_or_else(|_| panic!("Failed to execute tonic check for {}", example.path));

        let check_code = check_output.status.code().unwrap_or(-1);
        assert_eq!(
            check_code,
            example.check_exit,
            "[{}] check exit code mismatch. expected {}, got {}
stderr: {}",
            example.path,
            example.check_exit,
            check_code,
            String::from_utf8_lossy(&check_output.stderr)
        );

        // 2. tonic run
        let run_output = Command::new(env!("CARGO_BIN_EXE_tonic"))
            .args(["run", &example.path])
            .output()
            .unwrap_or_else(|_| panic!("Failed to execute tonic run for {}", example.path));

        let run_code = run_output.status.code().unwrap_or(-1);
        assert_eq!(
            run_code,
            example.run_exit,
            "[{}] run exit code mismatch. expected {}, got {}
stderr: {}",
            example.path,
            example.run_exit,
            run_code,
            String::from_utf8_lossy(&run_output.stderr)
        );

        // 3. check stdout
        if let Some(expected_stdout) = example.stdout {
            let actual_stdout = String::from_utf8(run_output.stdout).unwrap();
            assert_eq!(
                actual_stdout, expected_stdout,
                "[{}] stdout mismatch. expected {:?}, got {:?}",
                example.path, expected_stdout, actual_stdout
            );
        }

        // 4. check stderr
        if let Some(expected_stderr) = example.stderr_contains {
            let actual_stderr = String::from_utf8(run_output.stderr).unwrap();
            assert!(
                actual_stderr.contains(&expected_stderr),
                "[{}] stderr mismatch. expected it to contain {:?}, got {:?}",
                example.path,
                expected_stderr,
                actual_stderr
            );
        }
    }
}
