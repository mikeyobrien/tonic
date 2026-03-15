use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use std::fs;

mod common;

#[test]
fn compiled_runtime_supports_public_map_stdlib_surface_and_enum_into_map() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-map-stdlib");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    {\n      {Map.keys(%{a: 1, b: 2}), Map.values(%{a: 1, b: 2})},\n      {\n        {Map.merge(%{a: 1, b: 2}, %{b: 99, c: 3}), {Map.drop(%{a: 1, b: 2, c: 3}, [:a, :c]), Map.take(%{a: 1, b: 2, c: 3}, [:a, :c])}},\n        {\n          {{Map.get(%{a: 1}, :a, 0), Map.get(%{a: 1}, :z, \"fallback\")}, Map.put(%{a: 1}, :b, 2)},\n          {Map.delete(%{a: 1, b: 2}, :a), Enum.into([{:a, 1}, {:b, 2}], %{seed: 0})}\n        }\n      }\n    }\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["compile", "."])
        .assert()
        .success()
        .stdout(contains("compile: ok"));

    let executable = fixture_root.join(".tonic/build/main");
    assert!(
        executable.exists(),
        "expected compiled executable at {}",
        executable.display()
    );

    let output = std::process::Command::new(&executable)
        .current_dir(&fixture_root)
        .output()
        .expect("compiled executable should run");

    assert!(
        output.status.success(),
        "expected compiled executable success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "{{[:a, :b], [1, 2]}, {{%{:a => 1, :b => 99, :c => 3}, {%{:b => 2}, %{:a => 1, :c => 3}}}, {{{1, \"fallback\"}, %{:a => 1, :b => 2}}, {%{:b => 2}, %{:seed => 0, :a => 1, :b => 2}}}}}\n"
    );
}
