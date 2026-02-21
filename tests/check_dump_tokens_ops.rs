use std::fs;
use std::path::PathBuf;

#[test]
fn check_dump_tokens_supports_operator_and_atom_golden_stream() {
    let fixture_root = unique_fixture_root("check-dump-tokens-ops");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("lexer_ops.tn"),
        "defmodule Flow do\n  def run(value) do\n    case value do\n      :ok -> value |> wrap(:ok)\n      _ -> fn arg -> arg end\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write lexer operator fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/lexer_ops.tn", "--dump-tokens"])
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
        "IDENT(Flow)\n",
        "DO(do)\n",
        "DEF(def)\n",
        "IDENT(run)\n",
        "LPAREN\n",
        "IDENT(value)\n",
        "RPAREN\n",
        "DO(do)\n",
        "CASE(case)\n",
        "IDENT(value)\n",
        "DO(do)\n",
        "ATOM(ok)\n",
        "ARROW\n",
        "IDENT(value)\n",
        "PIPE_GT\n",
        "IDENT(wrap)\n",
        "LPAREN\n",
        "ATOM(ok)\n",
        "RPAREN\n",
        "IDENT(_)\n",
        "ARROW\n",
        "FN(fn)\n",
        "IDENT(arg)\n",
        "ARROW\n",
        "IDENT(arg)\n",
        "END(end)\n",
        "END(end)\n",
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
