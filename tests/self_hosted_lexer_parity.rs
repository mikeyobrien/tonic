use std::fs;
use std::path::{Path, PathBuf};

mod common;

#[test]
fn curated_self_hosted_lexer_fixture_matches_reference_dump() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    if let Err(mismatch) = common::self_hosted_lexer_parity::run_curated_corpus(
        Path::new(env!("CARGO_BIN_EXE_tonic")),
        &repo_root,
    ) {
        let artifact_root = common::unique_temp_dir("self-hosted-lexer-parity-mismatch");
        let source = fs::read_to_string(repo_root.join(&mismatch.fixture))
            .unwrap_or_else(|_| format!("# fixture source unavailable for {}\n", mismatch.fixture));
        let artifact_dir = common::self_hosted_lexer_parity::capture_mismatch_artifact(
            &artifact_root,
            &format!("parity-{}", mismatch.fixture.replace('/', "_")),
            &source,
            &mismatch,
        )
        .expect("mismatch artifact should be written");

        panic!(
            "self-hosted lexer parity mismatch for {}: {}. replay bundle: {}",
            mismatch.fixture,
            mismatch.reason,
            artifact_dir.display()
        );
    }
}

#[test]
fn forced_self_hosted_lexer_mismatch_writes_triage_artifacts() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = "tests/fixtures/self_hosted_lexer_parity/keywords_module.tn";
    let mut outputs = common::self_hosted_lexer_parity::collect_fixture_outputs(
        Path::new(env!("CARGO_BIN_EXE_tonic")),
        &repo_root,
        fixture,
    )
    .expect("fixture outputs should collect");

    outputs.self_hosted_tokens[0].lexeme = "forced-mismatch".to_string();

    let mismatch = common::self_hosted_lexer_parity::compare_fixture_outputs(&outputs)
        .expect_err("mutated token stream should mismatch");
    let artifact_root = common::unique_temp_dir("self-hosted-lexer-parity-artifact-contract");
    let source =
        fs::read_to_string(repo_root.join(fixture)).expect("fixture source should be readable");

    let artifact_dir = common::self_hosted_lexer_parity::capture_mismatch_artifact(
        &artifact_root,
        "forced-mismatch",
        &source,
        &mismatch,
    )
    .expect("mismatch artifact should be written");

    assert!(artifact_dir.join("program.tn").exists());
    assert!(artifact_dir.join("reference.json").exists());
    assert!(artifact_dir.join("self_hosted.json").exists());
    assert!(artifact_dir.join("mismatch.json").exists());

    let reference_dump = fs::read_to_string(artifact_dir.join("reference.json"))
        .expect("reference dump should be readable");
    let self_hosted_dump = fs::read_to_string(artifact_dir.join("self_hosted.json"))
        .expect("self-hosted dump should be readable");
    let mismatch_payload = fs::read_to_string(artifact_dir.join("mismatch.json"))
        .expect("mismatch payload should be readable");

    assert!(reference_dump.contains("DEFMODULE"));
    assert!(self_hosted_dump.contains("forced-mismatch"));
    assert!(mismatch_payload.contains(fixture));
    assert!(mismatch_payload.contains("token mismatch at index 0"));
}
