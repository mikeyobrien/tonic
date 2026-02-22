use std::fs;
use std::path::PathBuf;

#[test]
fn run_requires_lockfile_when_manifest_declares_dependencies() {
    let fixture_root = unique_fixture_root("run-dependency-lockfile-required");
    let project_root = fixture_root.join("app");
    let src_dir = project_root.join("src");
    let dep_root = fixture_root.join("shared_dep");

    fs::create_dir_all(&src_dir).expect("fixture setup should create project src directory");
    fs::create_dir_all(&dep_root).expect("fixture setup should create dependency directory");

    fs::write(
        project_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n\n[dependencies]\nshared_dep = { path = \"../shared_dep\" }\n",
    )
    .expect("fixture setup should write tonic.toml");

    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    Shared.answer()\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    fs::write(
        dep_root.join("shared.tn"),
        "defmodule Shared do\n  def answer() do\n    7\n  end\nend\n",
    )
    .expect("fixture setup should write dependency source");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&project_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: dependencies declared in tonic.toml but tonic.lock is missing; run `tonic deps lock` or `tonic deps sync`\n"
    );
}

#[test]
fn run_requires_warm_git_dependency_cache_when_lockfile_declares_git_deps() {
    let fixture_root = unique_fixture_root("run-dependency-git-cache-required");
    let project_root = fixture_root.join("app");
    let src_dir = project_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create project src directory");

    fs::write(
        project_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n\n[dependencies]\nremote_dep = { git = \"https://example.com/remote_dep.git\", rev = \"abc123\" }\n",
    )
    .expect("fixture setup should write tonic.toml");

    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    Remote.answer()\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    fs::write(
        project_root.join("tonic.lock"),
        "version = 1\n\n[path_deps]\n\n[git_deps.remote_dep]\nurl = \"https://example.com/remote_dep.git\"\nrev = \"abc123\"\n",
    )
    .expect("fixture setup should write tonic.lock");

    let expected_cache_dir = project_root.join(".tonic/deps/remote_dep");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(std::env::temp_dir())
        .arg("run")
        .arg(project_root.to_string_lossy().to_string())
        .output()
        .expect("run command should execute");

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        format!(
            "error: cached git dependency 'remote_dep' not found at {}; run `tonic deps sync`\n",
            expected_cache_dir.display()
        )
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
