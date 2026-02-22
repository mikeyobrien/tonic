use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn run_and_check_succeed_with_warm_git_cache_when_remote_is_unavailable() {
    let fixture_root = unique_fixture_root("deps-offline-warm-cache");
    let remote_repo = fixture_root.join("remote_dep");
    let project_root = fixture_root.join("app");
    let src_dir = project_root.join("src");

    fs::create_dir_all(&remote_repo).expect("fixture setup should create remote repo directory");
    fs::create_dir_all(&src_dir).expect("fixture setup should create project src directory");

    fs::write(
        remote_repo.join("remote.tn"),
        "defmodule Remote do\n  def answer() do\n    42\n  end\nend\n",
    )
    .expect("fixture setup should write remote dependency source");

    run_git(&remote_repo, ["init"]);
    run_git(&remote_repo, ["add", "."]);
    run_git(
        &remote_repo,
        [
            "-c",
            "user.email=deps-offline@test.invalid",
            "-c",
            "user.name=deps-offline",
            "commit",
            "-m",
            "initial",
        ],
    );

    let rev_output = std::process::Command::new("git")
        .current_dir(&remote_repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("fixture setup should resolve git revision");
    assert!(
        rev_output.status.success(),
        "expected rev-parse to succeed, stderr: {}",
        String::from_utf8_lossy(&rev_output.stderr)
    );
    let rev = String::from_utf8(rev_output.stdout)
        .expect("revision output should be utf8")
        .trim()
        .to_string();

    fs::write(
        project_root.join("tonic.toml"),
        format!(
            "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n\n[dependencies]\nremote_dep = {{ git = \"{}\", rev = \"{}\" }}\n",
            remote_repo.display(),
            rev
        ),
    )
    .expect("fixture setup should write tonic.toml");

    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    Remote.answer()\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let first_sync = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&project_root)
        .args(["deps", "sync"])
        .output()
        .expect("deps sync command should execute");
    assert!(
        first_sync.status.success(),
        "expected initial deps sync to succeed, status {:?}, stdout: {}, stderr: {}",
        first_sync.status.code(),
        String::from_utf8_lossy(&first_sync.stdout),
        String::from_utf8_lossy(&first_sync.stderr)
    );

    fs::remove_dir_all(&remote_repo)
        .expect("fixture setup should remove remote repo to simulate offline source");

    let second_sync = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&project_root)
        .args(["deps", "sync"])
        .output()
        .expect("deps sync command should execute with warm cache");
    assert!(
        second_sync.status.success(),
        "expected deps sync to succeed with warm cache, status {:?}, stdout: {}, stderr: {}",
        second_sync.status.code(),
        String::from_utf8_lossy(&second_sync.stdout),
        String::from_utf8_lossy(&second_sync.stderr)
    );

    let check_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&project_root)
        .args(["check", "."])
        .output()
        .expect("check command should execute with warm cache");
    assert!(
        check_output.status.success(),
        "expected check to succeed with warm cache, status {:?}, stdout: {}, stderr: {}",
        check_output.status.code(),
        String::from_utf8_lossy(&check_output.stdout),
        String::from_utf8_lossy(&check_output.stderr)
    );

    let run_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&project_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute with warm cache");

    assert!(
        run_output.status.success(),
        "expected run to succeed with warm cache, status {:?}, stdout: {}, stderr: {}",
        run_output.status.code(),
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );

    let run_stdout = String::from_utf8(run_output.stdout).expect("stdout should be utf8");
    assert_eq!(run_stdout, "42\n");
}

fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
    let output = std::process::Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("git command should run during fixture setup");

    assert!(
        output.status.success(),
        "expected git command {:?} to succeed, stderr: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
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
