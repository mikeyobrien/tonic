mod common;

use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn system_run_stream_true_emits_live_stdout_and_stderr_before_exit() {
    let fixture_root = common::unique_fixture_root("system-run-stream-live-output");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.run(\"printf 'stdout-live\\n'; printf 'stderr-live\\n' >&2; sleep 1; printf 'stdout-tail\\n'; printf 'stderr-tail\\n' >&2\", %{stream: true})\n    System.sleep_ms(200)\n    :done\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let mut child = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run command should spawn");

    let stdout = child
        .stdout
        .take()
        .expect("stdout pipe should be available");
    let stderr = child
        .stderr
        .take()
        .expect("stderr pipe should be available");

    let (stdout_first_tx, stdout_first_rx) = mpsc::channel();
    let stdout_thread = std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut first_line = String::new();
        reader
            .read_line(&mut first_line)
            .expect("stdout should yield first line");
        stdout_first_tx
            .send(first_line.clone())
            .expect("stdout first line should send");

        let mut rest = String::new();
        reader
            .read_to_string(&mut rest)
            .expect("stdout should remain readable");
        first_line + &rest
    });

    let (stderr_first_tx, stderr_first_rx) = mpsc::channel();
    let stderr_thread = std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut first_line = String::new();
        reader
            .read_line(&mut first_line)
            .expect("stderr should yield first line");
        stderr_first_tx
            .send(first_line.clone())
            .expect("stderr first line should send");

        let mut rest = String::new();
        reader
            .read_to_string(&mut rest)
            .expect("stderr should remain readable");
        first_line + &rest
    });

    let stdout_first = stdout_first_rx
        .recv_timeout(Duration::from_millis(800))
        .expect("stdout should stream before process exit");
    assert_eq!(stdout_first, "stdout-live\n");

    let stderr_first = stderr_first_rx
        .recv_timeout(Duration::from_millis(800))
        .expect("stderr should stream before process exit");
    assert_eq!(stderr_first, "stderr-live\n");

    assert!(
        child.try_wait().expect("try_wait should succeed").is_none(),
        "process should still be running after live output is observed"
    );

    let status = child.wait().expect("run command should finish");
    assert!(status.success(), "expected run success, got {status:?}");

    let stdout_all = stdout_thread.join().expect("stdout thread should finish");
    let stderr_all = stderr_thread.join().expect("stderr thread should finish");

    assert!(
        stdout_all.contains("stdout-tail\n"),
        "expected trailing stdout chunk, got: {stdout_all:?}"
    );
    assert!(
        stderr_all.contains("stderr-tail\n"),
        "expected trailing stderr chunk, got: {stderr_all:?}"
    );
}
