use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

mod common;

const DIFFERENTIAL_CATALOG_EXCLUSIONS: &[(&str, &str)] = &[
    (
        "examples/parity/02-operators/stepped_range.tn",
        "native C backend still aborts on tn_runtime_for-backed comprehensions",
    ),
    (
        "examples/parity/05-functions/function_capture_multi_clause_anon.tn",
        "native closure lowering still rejects this multi-clause anonymous function capture",
    ),
    (
        "examples/parity/06-control-flow/for_into_runtime_fail.tn",
        "native runtime failure output still lacks the interpreter's source-context-rich diagnostic text",
    ),
    (
        "examples/parity/10-idiomatic/closures_and_captures.tn",
        "native C backend still aborts on tn_runtime_for-backed comprehensions",
    ),
    (
        "examples/parity/10-idiomatic/fizzbuzz.tn",
        "native C backend still aborts on tn_runtime_for-backed comprehensions",
    ),
    (
        "examples/parity/10-idiomatic/keyword_filtering.tn",
        "native C backend still aborts on tn_runtime_for-backed comprehensions",
    ),
    (
        "examples/parity/10-idiomatic/list_processing.tn",
        "native C backend still aborts on tn_runtime_for-backed comprehensions",
    ),
    (
        "examples/parity/10-idiomatic/pipeline_transform.tn",
        "native C backend still aborts on tn_runtime_for-backed comprehensions",
    ),
];

#[derive(Debug, Deserialize)]
struct Catalog {
    example: Vec<CatalogEntry>,
}

#[derive(Debug, Deserialize)]
struct CatalogEntry {
    path: String,
    check_exit: i32,
    status: String,
}

#[test]
fn active_compileable_catalog_entries_have_explicit_differential_coverage() {
    let eligible_paths = active_compileable_catalog_paths();
    let eligible_set: BTreeSet<String> = eligible_paths.iter().cloned().collect();

    for (path, reason) in DIFFERENTIAL_CATALOG_EXCLUSIONS {
        assert!(
            eligible_set.contains(*path),
            "differential exclusion {path} must target an active catalog entry with check_exit = 0"
        );
        assert!(
            !reason.trim().is_empty(),
            "differential exclusion {path} must include a non-empty justification"
        );
    }

    let excluded: BTreeSet<String> = DIFFERENTIAL_CATALOG_EXCLUSIONS
        .iter()
        .map(|(path, _)| (*path).to_string())
        .collect();
    let included: BTreeSet<String> = differential_catalog_fixtures().into_iter().collect();
    let decided: BTreeSet<String> = included.union(&excluded).cloned().collect();
    let missing: Vec<String> = eligible_set.difference(&decided).cloned().collect();

    assert!(
        missing.is_empty(),
        "every active catalog entry with check_exit = 0 must be included in the differential gate or explicitly excluded: {missing:?}"
    );
}

#[test]
fn parity_catalog_fixtures_match_between_interpreter_and_native() {
    for fixture in differential_catalog_fixtures() {
        if let Err(mismatch) = common::differential::run_differential_fixture(
            Path::new(env!("CARGO_BIN_EXE_tonic")),
            Path::new("."),
            &fixture,
        ) {
            let artifact_root = common::unique_temp_dir("differential-parity-mismatch");
            let fixture_source = fs::read_to_string(&fixture)
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

fn differential_catalog_fixtures() -> Vec<String> {
    active_compileable_catalog_paths()
        .into_iter()
        .filter(|path| differential_exclusion_reason(path).is_none())
        .collect()
}

fn active_compileable_catalog_paths() -> Vec<String> {
    load_catalog()
        .example
        .into_iter()
        .filter(|entry| entry.status == "active" && entry.check_exit == 0)
        .map(|entry| entry.path)
        .collect()
}

fn differential_exclusion_reason(path: &str) -> Option<&'static str> {
    DIFFERENTIAL_CATALOG_EXCLUSIONS
        .iter()
        .find_map(|(excluded_path, reason)| (*excluded_path == path).then_some(*reason))
}

fn load_catalog() -> Catalog {
    toml::from_str(
        &fs::read_to_string("examples/parity/catalog.toml")
            .expect("parity catalog should be readable"),
    )
    .expect("parity catalog should parse")
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
