use std::fs;
mod common;

#[test]
fn check_dump_tokens_reports_map_fat_arrow_token() {
    let fixture_root = common::unique_fixture_root("check-dump-tokens-map-fat-arrow");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("map_fat_arrow_tokens.tn"),
        "defmodule Demo do\n  def run() do\n    %{\"status\" => 200}\n  end\nend\n",
    )
    .expect("fixture setup should write lexer fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/map_fat_arrow_tokens.tn", "--dump-tokens"])
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
        "IDENT(Demo)\n",
        "DO(do)\n",
        "DEF(def)\n",
        "IDENT(run)\n",
        "LPAREN\n",
        "RPAREN\n",
        "DO(do)\n",
        "PERCENT\n",
        "LBRACE\n",
        "STRING(status)\n",
        "FAT_ARROW\n",
        "INT(200)\n",
        "RBRACE\n",
        "END(end)\n",
        "END(end)\n",
        "EOF\n"
    );

    assert_eq!(stdout, expected);
}
