#![allow(dead_code)]

pub mod differential;
pub mod self_hosted_lexer_parity;

use std::path::PathBuf;

pub fn unique_fixture_root(test_name: &str) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    let path = std::env::temp_dir().join(format!(
        "tonic-{test_name}-{timestamp}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

pub fn unique_temp_dir(test_name: &str) -> PathBuf {
    unique_fixture_root(test_name)
}

/// Run a tonic subcommand and return the assert_cmd::Command for chaining.
pub fn tonic_cmd(args: &[&str]) -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::new(env!("CARGO_BIN_EXE_tonic"));
    cmd.args(args);
    cmd
}

/// Run tonic, assert exit 0, return stdout as String.
pub fn tonic_success(args: &[&str]) -> String {
    let output = tonic_cmd(args).assert().success().get_output().clone();
    String::from_utf8(output.stdout).unwrap()
}

/// Create an isolated TONIC_HOME in a temp directory.
/// Returns (temp_dir_path, tonic_home_path) where tonic_home_path = temp_dir/.tonic.
/// The temp directory is automatically cleaned up when the returned PathBuf's parent dir
/// is removed by the caller.
pub fn isolated_tonic_home(test_name: &str) -> (PathBuf, PathBuf) {
    let dir = unique_fixture_root(test_name);
    let home = dir.join(".tonic");
    std::fs::create_dir_all(&home).unwrap();
    (dir, home)
}
