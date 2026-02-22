use std::fs;
use std::path::PathBuf;

#[test]
fn check_dump_ast_matches_expression_contract_for_calls_and_precedence() {
    let fixture_root = unique_fixture_root("check-dump-ast-expressions");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("parser_expressions.tn"),
        "defmodule Math do\n  def compute() do\n    combine(1, 2) + wrap(inner(3 + 4))\n  end\nend\n",
    )
    .expect("fixture setup should write parser expression source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/parser_expressions.tn", "--dump-ast"])
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
        "{\"modules\":[{\"name\":\"Math\",\"functions\":[",
        "{\"name\":\"compute\",\"params\":[],\"body\":{",
        "\"kind\":\"binary\",\"op\":\"plus\",",
        "\"left\":{\"kind\":\"call\",\"callee\":\"combine\",\"args\":[",
        "{\"kind\":\"int\",\"value\":1},",
        "{\"kind\":\"int\",\"value\":2}",
        "]},",
        "\"right\":{\"kind\":\"call\",\"callee\":\"wrap\",\"args\":[",
        "{\"kind\":\"call\",\"callee\":\"inner\",\"args\":[",
        "{\"kind\":\"binary\",\"op\":\"plus\",",
        "\"left\":{\"kind\":\"int\",\"value\":3},",
        "\"right\":{\"kind\":\"int\",\"value\":4}}",
        "]}",
        "]}",
        "}}]}",
        "]}\n"
    );

    assert_eq!(stdout, expected);
}

#[test]
fn check_dump_ast_matches_primitive_literal_contract() {
    let fixture_root = unique_fixture_root("check-dump-ast-primitives");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("primitives.tn"),
        "defmodule Primitives do\n  def run() do\n    tuple(true, false) |> tuple(nil) |> tuple(\"hello\")\n  end\nend\n",
    )
    .expect("fixture setup should write primitives source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/primitives.tn", "--dump-ast"])
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
        "{\"modules\":[{\"name\":\"Primitives\",\"functions\":[",
        "{\"name\":\"run\",\"params\":[],\"body\":{",
        "\"kind\":\"pipe\",",
        "\"left\":{\"kind\":\"pipe\",",
        "\"left\":{\"kind\":\"call\",\"callee\":\"tuple\",\"args\":[",
        "{\"kind\":\"bool\",\"value\":true},",
        "{\"kind\":\"bool\",\"value\":false}",
        "]},",
        "\"right\":{\"kind\":\"call\",\"callee\":\"tuple\",\"args\":[",
        "{\"kind\":\"nil\"}",
        "]}",
        "},",
        "\"right\":{\"kind\":\"call\",\"callee\":\"tuple\",\"args\":[",
        "{\"kind\":\"string\",\"value\":\"hello\"}",
        "]}",
        "}}]}",
        "]}\n"
    );

    assert_eq!(stdout, expected);
}

#[test]
fn check_dump_ast_mul_binds_tighter_than_add() {
    let fixture_root = unique_fixture_root("check-dump-ast-mul-precedence");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("mul_precedence.tn"),
        "defmodule Math do\n  def compute() do\n    2 + 3 * 4\n  end\nend\n",
    )
    .expect("fixture setup should write mul precedence source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/mul_precedence.tn", "--dump-ast"])
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
        "{\"modules\":[{\"name\":\"Math\",\"functions\":[",
        "{\"name\":\"compute\",\"params\":[],\"body\":{",
        "\"kind\":\"binary\",\"op\":\"plus\",",
        "\"left\":{\"kind\":\"int\",\"value\":2},",
        "\"right\":{\"kind\":\"binary\",\"op\":\"mul\",",
        "\"left\":{\"kind\":\"int\",\"value\":3},",
        "\"right\":{\"kind\":\"int\",\"value\":4}",
        "}}}]}",
        "]}\n"
    );

    assert_eq!(stdout, expected);
}

#[test]
fn check_dump_ast_comparison_has_lower_precedence_than_arithmetic() {
    let fixture_root = unique_fixture_root("check-dump-ast-cmp-precedence");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("cmp_precedence.tn"),
        "defmodule Math do\n  def compute() do\n    4 - 1 > 2\n  end\nend\n",
    )
    .expect("fixture setup should write comparison precedence source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/cmp_precedence.tn", "--dump-ast"])
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
        "{\"modules\":[{\"name\":\"Math\",\"functions\":[",
        "{\"name\":\"compute\",\"params\":[],\"body\":{",
        "\"kind\":\"binary\",\"op\":\"gt\",",
        "\"left\":{\"kind\":\"binary\",\"op\":\"minus\",",
        "\"left\":{\"kind\":\"int\",\"value\":4},",
        "\"right\":{\"kind\":\"int\",\"value\":1}},",
        "\"right\":{\"kind\":\"int\",\"value\":2}",
        "}}]}",
        "]}\n"
    );

    assert_eq!(stdout, expected);
}

#[test]
fn check_dump_ast_logical_and_collection_precedence() {
    let fixture_root = unique_fixture_root("check-dump-ast-logical-collection");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("logical.tn"),
        "defmodule Math do\n  def compute() do\n    not 1 in 2..3 and 4 || 5\n  end\nend\n",
    )
    .expect("fixture setup should write comparison precedence source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/logical.tn", "--dump-ast"])
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
        "{\"modules\":[{\"name\":\"Math\",\"functions\":[",
        "{\"name\":\"compute\",\"params\":[],\"body\":{",
        "\"kind\":\"binary\",\"op\":\"oror\",",
        "\"left\":{\"kind\":\"binary\",\"op\":\"and\",",
        "\"left\":{\"kind\":\"binary\",\"op\":\"in\",",
        "\"left\":{\"kind\":\"unary\",\"op\":\"not\",",
        "\"value\":{\"kind\":\"int\",\"value\":1}},",
        "\"right\":{\"kind\":\"binary\",\"op\":\"range\",",
        "\"left\":{\"kind\":\"int\",\"value\":2},",
        "\"right\":{\"kind\":\"int\",\"value\":3}}},",
        "\"right\":{\"kind\":\"int\",\"value\":4}},",
        "\"right\":{\"kind\":\"int\",\"value\":5}",
        "}}]}",
        "]}\n"
    );

    assert_eq!(stdout, expected);
}

#[test]
fn check_dump_ast_concat_plus_plus_minus_minus_and_range_are_right_associative() {
    let fixture_root = unique_fixture_root("check-dump-ast-right-assoc");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("right_assoc.tn"),
        "defmodule Math do\n  def compute() do\n    tuple(\"a\" <> \"b\" <> \"c\", tuple(left ++ mid -- right, 1..2..3))\n  end\nend\n",
    )
    .expect("fixture setup should write right-associative source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/right_assoc.tn", "--dump-ast"])
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
        "{\"modules\":[{\"name\":\"Math\",\"functions\":[",
        "{\"name\":\"compute\",\"params\":[],\"body\":{",
        "\"kind\":\"call\",\"callee\":\"tuple\",\"args\":[",
        "{\"kind\":\"binary\",\"op\":\"concat\",",
        "\"left\":{\"kind\":\"string\",\"value\":\"a\"},",
        "\"right\":{\"kind\":\"binary\",\"op\":\"concat\",",
        "\"left\":{\"kind\":\"string\",\"value\":\"b\"},",
        "\"right\":{\"kind\":\"string\",\"value\":\"c\"}}},",
        "{\"kind\":\"call\",\"callee\":\"tuple\",\"args\":[",
        "{\"kind\":\"binary\",\"op\":\"plusplus\",",
        "\"left\":{\"kind\":\"variable\",\"name\":\"left\"},",
        "\"right\":{\"kind\":\"binary\",\"op\":\"minusminus\",",
        "\"left\":{\"kind\":\"variable\",\"name\":\"mid\"},",
        "\"right\":{\"kind\":\"variable\",\"name\":\"right\"}}},",
        "{\"kind\":\"binary\",\"op\":\"range\",",
        "\"left\":{\"kind\":\"int\",\"value\":1},",
        "\"right\":{\"kind\":\"binary\",\"op\":\"range\",",
        "\"left\":{\"kind\":\"int\",\"value\":2},",
        "\"right\":{\"kind\":\"int\",\"value\":3}}}",
        "]}",
        "]}}]}",
        "]}\n"
    );

    assert_eq!(stdout, expected);
}

#[test]
fn check_dump_ast_matches_collection_literal_contract() {
    let fixture_root = unique_fixture_root("check-dump-ast-collection-literals");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("collection_literals.tn"),
        "defmodule Demo do\n  def run() do\n    tuple({1, 2}, tuple([3, 4], tuple(%{ok: 5}, [done: 6])))\n  end\nend\n",
    )
    .expect("fixture setup should write collection literal source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/collection_literals.tn", "--dump-ast"])
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
        "{\"modules\":[{\"name\":\"Demo\",\"functions\":[",
        "{\"name\":\"run\",\"params\":[],\"body\":{",
        "\"kind\":\"call\",\"callee\":\"tuple\",\"args\":[",
        "{\"kind\":\"tuple\",\"items\":[",
        "{\"kind\":\"int\",\"value\":1},",
        "{\"kind\":\"int\",\"value\":2}",
        "]},",
        "{\"kind\":\"call\",\"callee\":\"tuple\",\"args\":[",
        "{\"kind\":\"list\",\"items\":[",
        "{\"kind\":\"int\",\"value\":3},",
        "{\"kind\":\"int\",\"value\":4}",
        "]},",
        "{\"kind\":\"call\",\"callee\":\"tuple\",\"args\":[",
        "{\"kind\":\"map\",\"entries\":[",
        "{\"key\":\"ok\",\"value\":{\"kind\":\"int\",\"value\":5}}",
        "]},",
        "{\"kind\":\"keyword\",\"entries\":[",
        "{\"key\":\"done\",\"value\":{\"kind\":\"int\",\"value\":6}}",
        "]}",
        "]}",
        "]}",
        "]}}]}",
        "]}\n"
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
