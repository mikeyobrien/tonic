mod common;

/// RED: numeric separators — 1_000_000 should lex as INTEGER(1000000)
#[test]
fn check_dump_tokens_integer_with_underscores() {
    let fixture_root = common::unique_fixture_root("check-tokens-numeric-sep");
    let examples_dir = fixture_root.join("examples");

    std::fs::create_dir_all(&examples_dir).unwrap();
    std::fs::write(
        examples_dir.join("num_sep.tn"),
        "defmodule Demo do\n  def run() do\n    1_000_000\n  end\nend\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/num_sep.tn", "--dump-tokens"])
        .output()
        .expect("tonic check should run");

    assert!(
        output.status.success(),
        "check should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    // The lexeme should have underscores stripped
    assert!(
        stdout.contains("INT(1000000)"),
        "expected INT(1000000) in token stream, got:\n{stdout}"
    );
}

/// RED: float with underscores — 1_000.50 should lex as FLOAT(1000.50)
#[test]
fn check_dump_tokens_float_with_underscores() {
    let fixture_root = common::unique_fixture_root("check-tokens-float-sep");
    let examples_dir = fixture_root.join("examples");

    std::fs::create_dir_all(&examples_dir).unwrap();
    std::fs::write(
        examples_dir.join("float_sep.tn"),
        "defmodule Demo do\n  def run() do\n    1_000.50\n  end\nend\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/float_sep.tn", "--dump-tokens"])
        .output()
        .expect("tonic check should run");

    assert!(
        output.status.success(),
        "check should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("FLOAT(1000.50)"),
        "expected FLOAT(1000.50) in token stream, got:\n{stdout}"
    );
}

/// RED: hex literal — 0xFF should lex as INTEGER(255)
#[test]
fn check_dump_tokens_hex_literal() {
    let fixture_root = common::unique_fixture_root("check-tokens-hex");
    let examples_dir = fixture_root.join("examples");

    std::fs::create_dir_all(&examples_dir).unwrap();
    std::fs::write(
        examples_dir.join("hex.tn"),
        "defmodule Demo do\n  def run() do\n    0xFF\n  end\nend\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/hex.tn", "--dump-tokens"])
        .output()
        .expect("tonic check should run");

    assert!(
        output.status.success(),
        "check should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("INT(255)"),
        "expected INT(255) in token stream, got:\n{stdout}"
    );
}

/// RED: octal literal — 0o77 should lex as INTEGER(63)
#[test]
fn check_dump_tokens_octal_literal() {
    let fixture_root = common::unique_fixture_root("check-tokens-octal");
    let examples_dir = fixture_root.join("examples");

    std::fs::create_dir_all(&examples_dir).unwrap();
    std::fs::write(
        examples_dir.join("octal.tn"),
        "defmodule Demo do\n  def run() do\n    0o77\n  end\nend\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/octal.tn", "--dump-tokens"])
        .output()
        .expect("tonic check should run");

    assert!(
        output.status.success(),
        "check should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("INT(63)"),
        "expected INT(63) in token stream, got:\n{stdout}"
    );
}

/// RED: binary literal — 0b1010 should lex as INTEGER(10)
#[test]
fn check_dump_tokens_binary_literal() {
    let fixture_root = common::unique_fixture_root("check-tokens-binary");
    let examples_dir = fixture_root.join("examples");

    std::fs::create_dir_all(&examples_dir).unwrap();
    std::fs::write(
        examples_dir.join("binary.tn"),
        "defmodule Demo do\n  def run() do\n    0b1010\n  end\nend\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/binary.tn", "--dump-tokens"])
        .output()
        .expect("tonic check should run");

    assert!(
        output.status.success(),
        "check should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("INT(10)"),
        "expected INT(10) in token stream, got:\n{stdout}"
    );
}

/// RED: char literal — ?a should lex as INTEGER(97)
#[test]
fn check_dump_tokens_char_literal() {
    let fixture_root = common::unique_fixture_root("check-tokens-char");
    let examples_dir = fixture_root.join("examples");

    std::fs::create_dir_all(&examples_dir).unwrap();
    std::fs::write(
        examples_dir.join("char.tn"),
        "defmodule Demo do\n  def run() do\n    ?a\n  end\nend\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/char.tn", "--dump-tokens"])
        .output()
        .expect("tonic check should run");

    assert!(
        output.status.success(),
        "check should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("INT(97)"),
        "expected INT(97) in token stream, got:\n{stdout}"
    );
}

/// RED: char escape literal — ?\n should lex as INTEGER(10)
#[test]
fn check_dump_tokens_char_escape_literal() {
    let fixture_root = common::unique_fixture_root("check-tokens-char-esc");
    let examples_dir = fixture_root.join("examples");

    std::fs::create_dir_all(&examples_dir).unwrap();
    std::fs::write(
        examples_dir.join("char_esc.tn"),
        "defmodule Demo do\n  def run() do\n    ?\\n\n  end\nend\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/char_esc.tn", "--dump-tokens"])
        .output()
        .expect("tonic check should run");

    assert!(
        output.status.success(),
        "check should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("INT(10)"),
        "expected INT(10) for ?\\n in token stream, got:\n{stdout}"
    );
}
