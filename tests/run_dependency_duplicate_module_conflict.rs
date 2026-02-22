use std::fs;
use std::path::PathBuf;

#[test]
fn run_rejects_duplicate_module_names_across_project_and_path_dependency() {
    let fixture_root = unique_fixture_root("run-duplicate-module-conflict");
    let src_dir = fixture_root.join("src");
    let dep_root = fixture_root.join("shared_dep");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::create_dir_all(&dep_root).expect("fixture setup should create dependency directory");

    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write project tonic.toml");

    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    Shared.answer()\n  end\nend\n",
    )
    .expect("fixture setup should write entry module");

    fs::write(
        src_dir.join("shared.tn"),
        "defmodule Shared do\n  def answer() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write project shared module");

    fs::write(
        dep_root.join("shared.tn"),
        "defmodule Shared do\n  def answer() do\n    2\n  end\nend\n",
    )
    .expect("fixture setup should write dependency shared module");

    let dep_path = dep_root
        .canonicalize()
        .expect("dependency path should canonicalize")
        .to_string_lossy()
        .replace('\\', "\\\\");

    let lockfile = format!(
        "version = 1\n\n[path_deps.shared_dep]\npath = \"{}\"\n\n[git_deps]\n",
        dep_path
    );

    fs::write(fixture_root.join("tonic.lock"), lockfile)
        .expect("fixture setup should write tonic.lock");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: [E1003] duplicate module definition 'Shared'\n"
    );
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
