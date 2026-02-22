use std::fs;
use std::path::PathBuf;

#[test]
fn deps_sync_reports_deterministic_diagnostic_for_unreachable_git_dependency() {
    let fixture_root = unique_fixture_root("deps-sync-git-failure-diagnostic");
    let project_root = fixture_root.join("app");
    let src_dir = project_root.join("src");
    let missing_repo = fixture_root.join("missing-remote");

    fs::create_dir_all(&src_dir).expect("fixture setup should create project src directory");

    fs::write(
        project_root.join("tonic.toml"),
        format!(
            "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n\n[dependencies]\nremote_dep = {{ git = \"{}\", rev = \"deadbeef\" }}\n",
            missing_repo.display()
        ),
    )
    .expect("fixture setup should write tonic.toml");

    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&project_root)
        .args(["deps", "sync"])
        .output()
        .expect("deps sync command should execute");

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "Syncing dependencies...\n");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        format!(
            "error: failed to sync dependencies: failed to fetch git dependency 'remote_dep' from '{}' at rev 'deadbeef'; verify the repository URL and revision are reachable\n",
            missing_repo.display()
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
