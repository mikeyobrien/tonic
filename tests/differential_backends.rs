use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

mod common;

const PARITY_DIFF_SUBSET: &[&str] = &[
    "examples/parity/02-operators/arithmetic_basic.tn",
    "examples/parity/02-operators/comparison_set.tn",
    "examples/parity/02-operators/membership_and_range.tn",
    "examples/parity/03-collections/list_literal.tn",
    "examples/parity/03-collections/map_literal_single_entry.tn",
    "examples/parity/03-collections/map_fat_arrow_literal.tn",
    "examples/parity/03-collections/tuple_literal_and_match.tn",
    "examples/parity/04-patterns/case_atom_and_wildcard.tn",
    "examples/parity/04-patterns/case_list_bind.tn",
    "examples/parity/04-patterns/pin_pattern_and_guard.tn",
    "examples/parity/05-functions/multi_clause_pattern_dispatch.tn",
    "examples/parity/06-control-flow/cond_branches.tn",
    "examples/parity/06-control-flow/with_happy_path.tn",
    "examples/parity/08-errors/question_operator_success.tn",
];

#[derive(Debug, Deserialize)]
struct Catalog {
    example: Vec<CatalogEntry>,
}

#[derive(Debug, Deserialize)]
struct CatalogEntry {
    path: String,
    status: String,
}

#[test]
fn parity_catalog_subset_matches_between_interpreter_and_native() {
    let catalog: Catalog = toml::from_str(
        &fs::read_to_string("examples/parity/catalog.toml")
            .expect("parity catalog should be readable"),
    )
    .expect("parity catalog should parse");

    let active_paths: BTreeSet<String> = catalog
        .example
        .into_iter()
        .filter(|entry| entry.status == "active")
        .map(|entry| entry.path)
        .collect();

    for fixture in PARITY_DIFF_SUBSET {
        assert!(
            active_paths.contains(*fixture),
            "fixture {fixture} must remain active in examples/parity/catalog.toml"
        );

        if let Err(mismatch) = common::differential::run_differential_fixture(
            Path::new(env!("CARGO_BIN_EXE_tonic")),
            Path::new("."),
            fixture,
        ) {
            let artifact_root = common::unique_temp_dir("differential-parity-mismatch");
            let fixture_source = fs::read_to_string(fixture)
                .unwrap_or_else(|_| format!("# fixture source unavailable for {fixture}\n"));
            let artifact_dir = common::differential::capture_mismatch_artifact(
                &artifact_root,
                &format!("parity-{}", fixture.replace('/', "_")),
                &fixture_source,
                &fixture_source,
                &mismatch,
            )
            .expect("mismatch artifact should be written");

            panic!(
                "differential mismatch for {fixture}: {}. replay bundle: {}",
                mismatch.reason,
                artifact_dir.display()
            );
        }
    }
}

#[test]
fn generated_programs_match_between_interpreter_and_native_with_replayable_seeds() {
    let fixture_root = common::unique_temp_dir("differential-fuzz");
    fs::create_dir_all(fixture_root.join("generated"))
        .expect("generated fixture directory should be created");

    for seed in differential_seeds() {
        let relative_path = format!("generated/seed-{seed}.tn");
        let absolute_path = fixture_root.join(&relative_path);
        let source = generate_program(seed);
        fs::write(&absolute_path, &source).expect("generated fixture should be written");

        let result = common::differential::run_differential_fixture(
            Path::new(env!("CARGO_BIN_EXE_tonic")),
            &fixture_root,
            &relative_path,
        );

        if let Err(mismatch) = result {
            let minimized = common::differential::minimize_source_by_lines(&source, |candidate| {
                fs::write(&absolute_path, candidate)
                    .expect("candidate fixture should be rewritable during minimization");
                common::differential::run_differential_fixture(
                    Path::new(env!("CARGO_BIN_EXE_tonic")),
                    &fixture_root,
                    &relative_path,
                )
                .is_err()
            });

            let artifact_dir = common::differential::capture_mismatch_artifact(
                &fixture_root,
                &format!("seed-{seed}"),
                &source,
                &minimized,
                &mismatch,
            )
            .expect("mismatch artifact should be written");

            panic!(
                "seed {seed} diverged: {}. replay bundle: {}",
                mismatch.reason,
                artifact_dir.display()
            );
        }
    }
}

#[test]
fn mismatch_artifact_capture_writes_replay_bundle() {
    let root = common::unique_temp_dir("differential-artifact-contract");
    let mismatch = common::differential::DifferentialMismatch {
        fixture: "generated/seed-99.tn".to_string(),
        reason: "stdout mismatch".to_string(),
        interpreter: common::differential::CommandOutcome {
            phase: "run".to_string(),
            exit_code: 0,
            stdout: "42\n".to_string(),
            stderr: String::new(),
            command: vec![
                "tonic".to_string(),
                "run".to_string(),
                "generated/seed-99.tn".to_string(),
            ],
        },
        native: common::differential::CommandOutcome {
            phase: "run".to_string(),
            exit_code: 0,
            stdout: "41\n".to_string(),
            stderr: String::new(),
            command: vec![
                "tonic".to_string(),
                "run".to_string(),
                ".tonic/build/seed-99.tnx.json".to_string(),
            ],
        },
    };

    let artifact_dir = common::differential::capture_mismatch_artifact(
        &root,
        "seed-99",
        "defmodule Demo do\n  def run() do\n    42\n  end\nend\n",
        "defmodule Demo do\n  def run() do\n    42\n  end\nend\n",
        &mismatch,
    )
    .expect("artifact capture should succeed");

    assert!(artifact_dir.join("program.tn").exists());
    assert!(artifact_dir.join("program.min.tn").exists());
    assert!(artifact_dir.join("mismatch.json").exists());

    let payload = fs::read_to_string(artifact_dir.join("mismatch.json"))
        .expect("mismatch payload should be readable");
    assert!(payload.contains("stdout mismatch"));
    assert!(payload.contains("generated/seed-99.tn"));
}

fn generate_program(seed: u64) -> String {
    let mut rng = Lcg::new(seed ^ 0xD1F5_5EED_u64);

    let selector = rng.next_range(3);
    let add_value = rng.next_range(7) as i64;
    let subtract_value = rng.next_range(7) as i64;
    let multiply_value = (rng.next_range(3) + 2) as i64;
    let expression = generate_expression(&mut rng, 3);

    format!(
        "defmodule Demo do\n  def pick(0, value) do\n    value + {add_value}\n  end\n\n  def pick(1, value) do\n    value - {subtract_value}\n  end\n\n  def pick(_, value) do\n    value * {multiply_value}\n  end\n\n  def run() do\n    pick({selector}, {expression})\n  end\nend\n"
    )
}

fn generate_expression(rng: &mut Lcg, depth: usize) -> String {
    if depth == 0 {
        return ((rng.next_range(11) as i64) - 5).to_string();
    }

    match rng.next_range(4) {
        0 => ((rng.next_range(11) as i64) - 5).to_string(),
        1 => format!(
            "({} + {})",
            generate_expression(rng, depth - 1),
            generate_expression(rng, depth - 1)
        ),
        2 => format!(
            "({} - {})",
            generate_expression(rng, depth - 1),
            generate_expression(rng, depth - 1)
        ),
        _ => format!(
            "({} * {})",
            generate_expression(rng, depth - 1),
            generate_expression(rng, depth - 1)
        ),
    }
}

fn differential_seeds() -> Vec<u64> {
    if let Ok(seed) = std::env::var("TONIC_DIFF_SEED") {
        let parsed = seed
            .parse::<u64>()
            .expect("TONIC_DIFF_SEED must be an unsigned integer");
        return vec![parsed];
    }

    let count = std::env::var("TONIC_DIFF_SEEDS")
        .ok()
        .map(|value| {
            value
                .parse::<u64>()
                .expect("TONIC_DIFF_SEEDS must be an unsigned integer")
        })
        .unwrap_or(32);

    (0..count).collect()
}

#[derive(Debug, Clone)]
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    fn next_range(&mut self, max_exclusive: u64) -> u64 {
        if max_exclusive == 0 {
            return 0;
        }

        self.next_u64() % max_exclusive
    }
}
