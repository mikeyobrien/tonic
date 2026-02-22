use std::fs;
mod common;

#[test]
fn run_resolves_manifest_path_dependencies_relative_to_project_root() {
    let fixture_root = common::unique_fixture_root("run-path-dependency-external-cwd");
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

    let dep_path = dep_root
        .canonicalize()
        .expect("dependency path should canonicalize")
        .to_string_lossy()
        .replace('\\', "\\\\");

    let lockfile = format!(
        "version = 1\n\n[path_deps.shared_dep]\npath = \"{}\"\n\n[git_deps]\n",
        dep_path
    );
    fs::write(project_root.join("tonic.lock"), lockfile)
        .expect("fixture setup should write tonic.lock");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(std::env::temp_dir())
        .arg("run")
        .arg(project_root.to_string_lossy().to_string())
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected run command to succeed, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "7\n");
}
