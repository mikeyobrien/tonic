use serde_json::Value;
use std::fs;
use std::process::Command;

mod common;

#[test]
fn harness_writes_reports_and_enforce_fails_on_mismatch() {
    let root = common::unique_temp_dir("llvm-catalog-parity-harness");
    let catalog_path = root.join("catalog.toml");
    let report_json = root.join("report.json");
    let report_md = root.join("report.md");
    let tonic_bin = root.join("fake-tonic.sh");

    fs::create_dir_all(root.join("fixtures")).expect("fixture dir should exist");
    fs::write(root.join("fixtures/pass.tn"), "# fixture\n").expect("fixture should write");
    fs::write(root.join("fixtures/compile_fail.tn"), "# fixture\n").expect("fixture should write");
    fs::write(root.join("fixtures/runtime_fail.tn"), "# fixture\n").expect("fixture should write");

    fs::write(
        &catalog_path,
        r#"[[example]]
path = "fixtures/pass.tn"
check_exit = 0
run_exit = 0
stdout = '''ok-pass
'''
status = "active"

[[example]]
path = "fixtures/compile_fail.tn"
check_exit = 0
run_exit = 0
stdout = '''never
'''
status = "active"

[[example]]
path = "fixtures/runtime_fail.tn"
check_exit = 0
run_exit = 0
stdout = '''expected
'''
status = "active"
"#,
    )
    .expect("catalog should write");

    fs::write(
        &tonic_bin,
        r#"#!/usr/bin/env bash
set -euo pipefail

if [[ "$1" != "compile" ]]; then
  printf 'error: unsupported fake command\n' >&2
  exit 9
fi

fixture="$2"
name="$(basename "$fixture")"
name="${name%.tn}"

if [[ "$name" == "compile_fail" ]]; then
  printf 'error: synthetic compile failure\n' >&2
  exit 1
fi

artifact_dir="$PWD/.fake-native"
mkdir -p "$artifact_dir"
artifact="$artifact_dir/$name"

if [[ "$name" == "runtime_fail" ]]; then
  cat > "$artifact" <<'ART'
#!/usr/bin/env bash
printf 'actual\n'
ART
else
  cat > "$artifact" <<'ART'
#!/usr/bin/env bash
printf 'ok-pass\n'
ART
fi
chmod +x "$artifact"
printf 'compile: ok %s\n' "$artifact"
"#,
    )
    .expect("fake tonic script should write");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&tonic_bin)
            .expect("fake tonic metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&tonic_bin, perms).expect("fake tonic should be executable");
    }

    let output = Command::new(env!("CARGO_BIN_EXE_llvm_catalog_parity"))
        .current_dir(&root)
        .args([
            "--catalog",
            catalog_path.to_str().expect("catalog path utf8"),
            "--tonic-bin",
            tonic_bin.to_str().expect("tonic path utf8"),
            "--report-json",
            report_json.to_str().expect("json path utf8"),
            "--report-md",
            report_md.to_str().expect("md path utf8"),
        ])
        .output()
        .expect("harness should execute");

    assert!(
        output.status.success(),
        "non-enforce mode should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(report_json.exists(), "json report should exist");
    assert!(report_md.exists(), "markdown report should exist");

    let json: Value = serde_json::from_str(
        &fs::read_to_string(&report_json).expect("json report should be readable"),
    )
    .expect("json report should parse");

    assert_eq!(json["summary"]["compile_matches"], 2);
    assert_eq!(json["summary"]["compile_mismatches"], 1);
    assert_eq!(json["summary"]["runtime_matches"], 1);
    assert_eq!(json["summary"]["runtime_mismatches"], 1);

    let markdown = fs::read_to_string(&report_md).expect("md report should be readable");
    assert!(markdown.contains("# LLVM Catalog Parity Report"));
    assert!(markdown.contains("Top failure causes"));

    let enforce = Command::new(env!("CARGO_BIN_EXE_llvm_catalog_parity"))
        .current_dir(&root)
        .args([
            "--catalog",
            catalog_path.to_str().expect("catalog path utf8"),
            "--tonic-bin",
            tonic_bin.to_str().expect("tonic path utf8"),
            "--report-json",
            report_json.to_str().expect("json path utf8"),
            "--report-md",
            report_md.to_str().expect("md path utf8"),
            "--enforce",
        ])
        .output()
        .expect("enforce harness should execute");

    assert!(
        !enforce.status.success(),
        "enforce mode should fail while mismatches exist"
    );
}
