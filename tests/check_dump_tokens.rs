use std::fs;
use std::path::PathBuf;

#[test]
fn check_dump_tokens_matches_minimal_module_golden_stream() {
    let fixture_root = unique_fixture_root("check-dump-tokens");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("lexer_smoke.tn"),
        "defmodule Math do\n  def add(a, b) do\n    a + b\n  end\nend\n",
    )
    .expect("fixture setup should write lexer smoke source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/lexer_smoke.tn", "--dump-tokens"])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected successful check invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let expected = concat!(
        "DEFMODULE(defmodule)\n",
        "IDENT(Math)\n",
        "DO(do)\n",
        "DEF(def)\n",
        "IDENT(add)\n",
        "LPAREN\n",
        "IDENT(a)\n",
        "COMMA\n",
        "IDENT(b)\n",
        "RPAREN\n",
        "DO(do)\n",
        "IDENT(a)\n",
        "PLUS\n",
        "IDENT(b)\n",
        "END(end)\n",
        "END(end)\n",
        "EOF\n"
    );

    assert_eq!(stdout, expected);
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
