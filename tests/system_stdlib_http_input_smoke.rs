use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};

mod common;

#[test]
fn run_system_read_text_reads_file_content() {
    let fixture_root = common::unique_fixture_root("system-read-text");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.read_text(\"payload.txt\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");
    fs::write(fixture_root.join("payload.txt"), "hello from file")
        .expect("fixture setup should write payload");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected run success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "\"hello from file\"\n"
    );
}

#[test]
fn run_system_list_files_recursive_returns_sorted_nested_paths_for_spaced_directory() {
    let fixture_root = common::unique_fixture_root("system-list-files-recursive");
    let src_dir = fixture_root.join("src");
    let assets_dir = fixture_root.join("assets with space").join("docs");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::create_dir_all(&assets_dir).expect("fixture setup should create nested asset directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.list_files_recursive(\"assets with space\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");
    fs::write(
        fixture_root.join("assets with space").join("style.css"),
        "root",
    )
    .expect("fixture setup should write root asset");
    fs::write(assets_dir.join("guide.css"), "nested")
        .expect("fixture setup should write nested asset");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected run success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("\"docs/guide.css\"") && stdout.contains("\"style.css\""),
        "expected nested + root file paths in output, got: {stdout}"
    );
    assert!(
        stdout.find("docs/guide.css").unwrap_or(usize::MAX)
            < stdout.find("style.css").unwrap_or(usize::MAX),
        "expected deterministic sorted order, got: {stdout}"
    );
}

#[test]
fn run_system_remove_tree_removes_spaced_nested_directory() {
    let fixture_root = common::unique_fixture_root("system-remove-tree");
    let src_dir = fixture_root.join("src");
    let output_dir = fixture_root.join("out with space").join("docs");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::create_dir_all(&output_dir).expect("fixture setup should create nested output directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    {System.remove_tree(\"out with space\"), System.remove_tree(\"out with space\")}\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");
    fs::write(output_dir.join("guide.css"), "nested")
        .expect("fixture setup should write nested output file");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected run success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "{true, false}\n"
    );
    assert!(
        !fixture_root.join("out with space").exists(),
        "expected remove_tree target to be gone"
    );
}

#[test]
fn run_system_read_stdin_reads_piped_input() {
    let fixture_root = common::unique_fixture_root("system-read-stdin");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.read_stdin()\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let mut child = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run command should spawn");

    let mut stdin = child.stdin.take().expect("stdin pipe should be available");
    stdin
        .write_all(b"piped input")
        .expect("stdin write should succeed");
    drop(stdin);

    let output = child
        .wait_with_output()
        .expect("run command should complete");

    assert!(
        output.status.success(),
        "expected run success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "\"piped input\"\n"
    );
}

#[test]
fn run_system_http_request_returns_expected_map_shape() {
    let fixture_root = common::unique_fixture_root("system-http-request-success");
    let src_dir = fixture_root.join("src");

    let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
    let addr = listener
        .local_addr()
        .expect("listener should expose address");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .expect("server should accept one connection");

        let mut request_buf = [0u8; 1024];
        let _ = stream.read(&mut request_buf);

        let body = "hello";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nX-Test: yep\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("server should write response");
    });

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        format!(
            "defmodule Demo do\n  def run() do\n    System.http_request(\"GET\", \"http://{addr}/demo\", [], \"\", %{{}})\n  end\nend\n"
        ),
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    server.join().expect("server thread should finish");

    assert!(
        output.status.success(),
        "expected run success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains(":status => 200"),
        "expected status key in response map, got: {stdout}"
    );
    assert!(
        stdout.contains(":headers => ["),
        "expected headers list in response map, got: {stdout}"
    );
    assert!(
        stdout.contains("{\"content-type\", \"text/plain\"}"),
        "expected lowercase content-type header tuple, got: {stdout}"
    );
    assert!(
        stdout.contains(":body => \"hello\""),
        "expected body key in response map, got: {stdout}"
    );
    assert!(
        stdout.contains(":final_url => \"http://"),
        "expected final_url key in response map, got: {stdout}"
    );
}

#[test]
fn run_system_read_stdin_returns_empty_string_for_empty_input() {
    let fixture_root = common::unique_fixture_root("system-read-stdin-empty");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.read_stdin()\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let mut child = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run command should spawn");

    // Close stdin immediately without writing — empty input
    drop(child.stdin.take());

    let output = child
        .wait_with_output()
        .expect("run command should complete");

    assert!(
        output.status.success(),
        "expected run success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "\"\"\n"
    );
}

#[test]
fn run_system_read_text_rejects_non_string_argument_deterministically() {
    let fixture_root = common::unique_fixture_root("system-read-text-type-error");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.read_text(42)\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run failure for wrong argument type"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_read_text expects string argument 1; found int"),
        "expected deterministic type-error message, got: {stderr}"
    );
}

#[test]
fn run_system_http_request_rejects_invalid_method_deterministically() {
    let fixture_root = common::unique_fixture_root("system-http-request-method-error");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.http_request(\"TRACE\", \"https://example.com\", [], \"\", %{})\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run failure for unsupported method"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_http_request invalid method: TRACE"),
        "expected deterministic invalid-method error, got: {stderr}"
    );
}

#[test]
fn run_system_http_request_rejects_timeout_out_of_range_deterministically() {
    let fixture_root = common::unique_fixture_root("system-http-request-timeout-error");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.http_request(\"GET\", \"https://example.com\", [], \"\", %{timeout_ms: 10})\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run failure for timeout out of range"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_http_request timeout_ms out of range: 10"),
        "expected deterministic timeout-range error, got: {stderr}"
    );
}

#[test]
fn run_system_http_request_rejects_unsupported_url_scheme_deterministically() {
    let fixture_root = common::unique_fixture_root("system-http-request-scheme-error");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.http_request(\"GET\", \"ftp://example.com\", [], \"\", %{})\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run failure for unsupported scheme"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_http_request unsupported url scheme: ftp"),
        "expected deterministic unsupported-scheme error, got: {stderr}"
    );
}

#[test]
fn run_system_list_files_recursive_skips_symlinked_entries() {
    let fixture_root = common::unique_fixture_root("system-list-files-symlink");
    let src_dir = fixture_root.join("src");
    let real_dir = fixture_root.join("tree").join("real");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::create_dir_all(&real_dir).expect("fixture setup should create real sub-directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.list_files_recursive(\"tree\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");
    fs::write(fixture_root.join("tree").join("root.txt"), "root")
        .expect("fixture setup should write root file");
    fs::write(real_dir.join("nested.txt"), "nested")
        .expect("fixture setup should write nested file");

    // Create a symlink to a file and a symlink to a directory inside the tree.
    let link_file = fixture_root.join("tree").join("linkfile.txt");
    let link_dir = fixture_root.join("tree").join("linkdir");
    std::os::unix::fs::symlink(fixture_root.join("tree").join("root.txt"), &link_file)
        .expect("fixture setup should create symlink to file");
    std::os::unix::fs::symlink(&real_dir, &link_dir)
        .expect("fixture setup should create symlink to directory");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected run success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    // Only real files should appear; symlinked file and symlinked directory are skipped.
    assert!(
        stdout.contains("\"real/nested.txt\"") && stdout.contains("\"root.txt\""),
        "expected only real file paths in output, got: {stdout}"
    );
    assert!(
        !stdout.contains("linkfile.txt"),
        "expected symlinked file to be excluded from output, got: {stdout}"
    );
    assert!(
        !stdout.contains("linkdir"),
        "expected symlinked directory contents to be excluded from output, got: {stdout}"
    );
}

#[test]
fn run_system_list_files_recursive_errors_on_missing_path() {
    let fixture_root = common::unique_fixture_root("system-list-files-missing");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.list_files_recursive(\"no_such_dir\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run failure for missing path"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_list_files_recursive failed for 'no_such_dir'"),
        "expected deterministic missing-path error, got: {stderr}"
    );
}

#[test]
fn run_system_list_files_recursive_rejects_non_string_argument() {
    let fixture_root = common::unique_fixture_root("system-list-files-type-error");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.list_files_recursive(42)\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run failure for wrong argument type"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: host error: sys_list_files_recursive expects string argument 1; found int"
        ),
        "expected deterministic type-error message, got: {stderr}"
    );
}

#[test]
fn run_system_list_files_recursive_rejects_empty_path() {
    let fixture_root = common::unique_fixture_root("system-list-files-empty-path");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.list_files_recursive(\"\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run failure for empty path"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_list_files_recursive path must not be empty"),
        "expected deterministic empty-path error, got: {stderr}"
    );
}

#[test]
fn run_system_remove_tree_removes_symlinked_file_as_file() {
    let fixture_root = common::unique_fixture_root("system-remove-tree-symlink-file");
    let src_dir = fixture_root.join("src");
    let target_file = fixture_root.join("real.txt");
    let link_file = fixture_root.join("linkfile.txt");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(&target_file, "real content").expect("fixture setup should write real file");
    std::os::unix::fs::symlink(&target_file, &link_file)
        .expect("fixture setup should create symlink to file");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.remove_tree(\"linkfile.txt\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected run success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "true\n",
        "expected true when symlink was removed"
    );
    // Symlink is gone; the real file must survive.
    assert!(
        link_file.symlink_metadata().is_err(),
        "expected symlink to be removed"
    );
    assert!(target_file.exists(), "expected real file to survive");
}

#[test]
fn run_system_remove_tree_on_symlinked_directory_removes_symlink_only() {
    let fixture_root = common::unique_fixture_root("system-remove-tree-symlink-dir");
    let src_dir = fixture_root.join("src");
    let real_dir = fixture_root.join("realdir");
    let link_dir = fixture_root.join("linkdir");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::create_dir_all(&real_dir).expect("fixture setup should create real directory");
    fs::write(real_dir.join("inside.txt"), "content")
        .expect("fixture setup should write file inside real directory");
    std::os::unix::fs::symlink(&real_dir, &link_dir)
        .expect("fixture setup should create symlink to directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.remove_tree(\"linkdir\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected run success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "true\n",
        "expected true when symlink-to-directory was removed"
    );
    // Symlink is gone; real directory and its contents must survive.
    assert!(
        link_dir.symlink_metadata().is_err(),
        "expected symlink to directory to be removed"
    );
    assert!(
        real_dir.exists(),
        "expected real directory to survive after symlink removal"
    );
    assert!(
        real_dir.join("inside.txt").exists(),
        "expected real directory contents to survive"
    );
}

#[test]
fn run_system_remove_tree_rejects_non_string_argument() {
    let fixture_root = common::unique_fixture_root("system-remove-tree-type-error");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.remove_tree(42)\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run failure for wrong argument type"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_remove_tree expects string argument 1; found int"),
        "expected deterministic type-error message, got: {stderr}"
    );
}
