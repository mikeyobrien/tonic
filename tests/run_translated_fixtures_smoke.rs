use std::fs;
use std::path::PathBuf;

#[test]
fn translated_fixtures_check_and_run() {
    let fixture_root = unique_fixture_root("translated-fixtures");
    let examples_dir = fixture_root.join("examples").join("translated");
    fs::create_dir_all(&examples_dir).expect("fixture setup should create translated examples dir");

    fs::write(
        examples_dir.join("control_flow.tn"),
        "defmodule Demo do\n  def run() do\n    with [left, right] <- [20, 22],\n         sum <- left + right do\n      if sum > 40 do\n        sum\n      else\n        0\n      end\n    else\n      _ -> 0\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write control flow translation fixture");

    fs::write(
        examples_dir.join("module_forms.tn"),
        "defmodule Math do\n  def double(value) do\n    value * 2\n  end\nend\n\ndefmodule Demo do\n  alias Math, as: M\n  import Math\n\n  def run() do\n    M.double(10) + double(11)\n  end\nend\n",
    )
    .expect("fixture setup should write module forms translation fixture");

    for (path, expected_stdout) in [
        ("examples/translated/control_flow.tn", "42\n"),
        ("examples/translated/module_forms.tn", "42\n"),
    ] {
        let check_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
            .current_dir(&fixture_root)
            .args(["check", path])
            .output()
            .expect("check command should execute for translated fixture");

        assert!(
            check_output.status.success(),
            "expected check to succeed for {}, got status {:?} and stderr: {}",
            path,
            check_output.status.code(),
            String::from_utf8_lossy(&check_output.stderr)
        );

        let run_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
            .current_dir(&fixture_root)
            .args(["run", path])
            .output()
            .expect("run command should execute for translated fixture");

        assert!(
            run_output.status.success(),
            "expected run to succeed for {}, got status {:?} and stderr: {}",
            path,
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );

        let stdout = String::from_utf8(run_output.stdout).expect("stdout should be utf8");
        assert_eq!(
            stdout, expected_stdout,
            "unexpected output for fixture {path}"
        );
    }
}

fn unique_fixture_root(test_name: &str) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "tonic-{test_name}-{timestamp}-{}",
        std::process::id()
    ))
}
