use std::fs;
use std::path::Path;
mod common;

#[test]
fn deps_sync_fetches_local_git_dependency_and_run_uses_it() {
    let fixture_root = common::unique_fixture_root("deps-sync-git-local");
    let remote_repo = fixture_root.join("remote_dep");
    let project_root = fixture_root.join("app");
    let src_dir = project_root.join("src");

    fs::create_dir_all(&remote_repo).expect("fixture setup should create remote repo directory");
    fs::create_dir_all(&src_dir).expect("fixture setup should create project src directory");

    fs::write(
        remote_repo.join("remote.tn"),
        "defmodule Remote do\n  def answer() do\n    9\n  end\nend\n",
    )
    .expect("fixture setup should write remote dependency source");

    run_git(&remote_repo, ["init"]);
    run_git(&remote_repo, ["add", "."]);
    run_git(
        &remote_repo,
        [
            "-c",
            "user.email=deps-sync@test.invalid",
            "-c",
            "user.name=deps-sync",
            "commit",
            "-m",
            "initial",
        ],
    );

    let rev = std::process::Command::new("git")
        .current_dir(&remote_repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("fixture setup should resolve git revision");
    assert!(
        rev.status.success(),
        "expected rev-parse to succeed, stderr: {}",
        String::from_utf8_lossy(&rev.stderr)
    );
    let rev = String::from_utf8(rev.stdout)
        .expect("rev stdout should be utf8")
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

    let sync_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&project_root)
        .args(["deps", "sync"])
        .output()
        .expect("deps sync command should execute");

    assert!(
        sync_output.status.success(),
        "expected deps sync to succeed, status {:?}, stdout: {}, stderr: {}",
        sync_output.status.code(),
        String::from_utf8_lossy(&sync_output.stdout),
        String::from_utf8_lossy(&sync_output.stderr)
    );

    let run_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&project_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        run_output.status.success(),
        "expected run to succeed, status {:?}, stderr: {}",
        run_output.status.code(),
        String::from_utf8_lossy(&run_output.stderr)
    );

    let stdout = String::from_utf8(run_output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "9\n");
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
