use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod common;

#[test]
fn release_workflow_runs_alpha_readiness_gate() {
    let workflow_path = repo_root().join(".github/workflows/release-native-benchmarks.yml");
    let workflow = fs::read_to_string(&workflow_path).expect("workflow should exist");

    assert!(workflow.contains("./scripts/release-alpha-readiness.sh"));
    assert!(workflow.contains("TONIC_ALPHA_ARTIFACT_DIR: .tonic/release"));
    assert!(workflow.contains("path: .tonic/release/"));
}

#[test]
fn release_checklist_documents_one_shot_alpha_gate() {
    let checklist_path = repo_root().join("docs/release-checklist.md");
    let checklist = fs::read_to_string(&checklist_path).expect("release checklist should exist");

    assert!(checklist.contains("./scripts/release-alpha-readiness.sh --version X.Y.Z-alpha.N"));
}

#[test]
fn alpha_readiness_fails_when_changelog_is_missing() {
    let fixture = setup_fixture(
        "alpha-readiness-missing-changelog",
        None,
        StubBehavior::PassWithArtifacts,
    );

    let output = run_alpha_readiness(&fixture.root, &fixture.native_gates_cmd);

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected deterministic failure exit code when changelog is missing; stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("CHANGELOG.md not found"),
        "expected missing changelog diagnostic, got: {stderr}"
    );
}

#[test]
fn alpha_readiness_fails_when_version_heading_is_missing() {
    let fixture = setup_fixture(
        "alpha-readiness-missing-version-heading",
        Some("# Changelog\n\n## [0.1.0-alpha.0] - 2026-02-24\n\n- prior alpha entry\n"),
        StubBehavior::PassWithArtifacts,
    );

    let output = run_alpha_readiness(&fixture.root, &fixture.native_gates_cmd);

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected deterministic failure exit code when changelog heading is missing; stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("CHANGELOG.md is missing version heading: ## [0.1.0-alpha.1]"),
        "expected missing heading diagnostic, got: {stderr}"
    );
}

#[test]
fn alpha_readiness_fails_when_native_gates_command_fails() {
    let fixture = setup_fixture(
        "alpha-readiness-native-gates-fail",
        Some("# Changelog\n\n## [0.1.0-alpha.1] - 2026-02-25\n\n- alpha candidate\n"),
        StubBehavior::Fail,
    );

    let output = run_alpha_readiness(&fixture.root, &fixture.native_gates_cmd);

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected deterministic failure exit code when native gates fail; stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("native gates command failed"),
        "expected native gate failure diagnostic, got: {stderr}"
    );
}

#[test]
fn alpha_readiness_passes_with_matching_changelog_and_artifacts() {
    let fixture = setup_fixture(
        "alpha-readiness-pass",
        Some("# Changelog\n\n## [0.1.0-alpha.1] - 2026-02-25\n\n- alpha candidate\n"),
        StubBehavior::PassWithArtifacts,
    );

    let output = run_alpha_readiness(&fixture.root, &fixture.native_gates_cmd);

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected pass exit code; stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("alpha-readiness: pass: alpha release readiness checks succeeded"),
        "expected pass marker in stdout, got: {stdout}"
    );

    let artifact_root = fixture.root.join(".tonic/native-gates");
    assert!(artifact_root.join("native-compiler-summary.json").exists());
    assert!(artifact_root.join("native-compiler-summary.md").exists());
    assert!(artifact_root.join("native-compiled-summary.json").exists());
    assert!(artifact_root.join("native-compiled-summary.md").exists());
}

fn run_alpha_readiness(root: &Path, native_gates_cmd: &Path) -> Output {
    Command::new("bash")
        .arg(readiness_script_path())
        .arg("--version")
        .arg("0.1.0-alpha.1")
        .current_dir(root)
        .env("TONIC_NATIVE_GATES_CMD", native_gates_cmd)
        .output()
        .expect("alpha readiness script should execute")
}

#[derive(Clone, Copy)]
enum StubBehavior {
    PassWithArtifacts,
    Fail,
}

struct Fixture {
    root: PathBuf,
    native_gates_cmd: PathBuf,
}

fn setup_fixture(test_name: &str, changelog: Option<&str>, behavior: StubBehavior) -> Fixture {
    let root = common::unique_fixture_root(test_name);

    fs::create_dir_all(root.join("scripts")).expect("fixture should create scripts directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0-alpha.1\"\nedition = \"2021\"\n",
    )
    .expect("fixture should write Cargo.toml");

    if let Some(changelog_content) = changelog {
        fs::write(root.join("CHANGELOG.md"), changelog_content)
            .expect("fixture should write changelog");
    }

    let native_gates_cmd = root.join("scripts/fake-native-gates.sh");
    fs::write(&native_gates_cmd, stub_script_content(behavior)).expect("fixture should write stub");
    make_executable(&native_gates_cmd);

    init_git_repo(&root);

    Fixture {
        root,
        native_gates_cmd,
    }
}

fn init_git_repo(root: &Path) {
    run_git(root, &["init", "-q"]);
    run_git(root, &["config", "user.name", "Tonic Test"]);
    run_git(root, &["config", "user.email", "tonic-test@example.com"]);
    run_git(root, &["add", "."]);
    run_git(root, &["commit", "-qm", "fixture init"]);
}

fn run_git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git command should execute");

    assert!(
        output.status.success(),
        "git {:?} should succeed; stdout: {}, stderr: {}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn stub_script_content(behavior: StubBehavior) -> &'static str {
    match behavior {
        StubBehavior::PassWithArtifacts => {
            "#!/usr/bin/env bash\nset -euo pipefail\nartifact_dir=\"${TONIC_NATIVE_ARTIFACT_DIR:-.tonic/native-gates}\"\nmkdir -p \"$artifact_dir\"\nprintf '%s\\n' '{}' > \"$artifact_dir/native-compiler-summary.json\"\nprintf '%s\\n' '# native compiler summary' > \"$artifact_dir/native-compiler-summary.md\"\nprintf '%s\\n' '{}' > \"$artifact_dir/native-compiled-summary.json\"\nprintf '%s\\n' '# native compiled summary' > \"$artifact_dir/native-compiled-summary.md\"\n"
        }
        StubBehavior::Fail => {
            "#!/usr/bin/env bash\nset -euo pipefail\nprintf '%s\\n' 'simulated native gate failure' >&2\nexit 7\n"
        }
    }
}

fn readiness_script_path() -> PathBuf {
    repo_root().join("scripts/release-alpha-readiness.sh")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path)
        .expect("executable path should exist")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("should set executable bit");
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}
