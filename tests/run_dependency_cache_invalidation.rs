use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn run_cache_misses_when_lockfile_dependency_graph_changes() {
    let fixture_root = unique_fixture_root("run-dependency-cache-invalidation");
    let src_dir = fixture_root.join("src");
    let dep_v1 = fixture_root.join("deps/shared_dep_v1");
    let dep_v2 = fixture_root.join("deps/shared_dep_v2");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::create_dir_all(&dep_v1).expect("fixture setup should create dependency v1 directory");
    fs::create_dir_all(&dep_v2).expect("fixture setup should create dependency v2 directory");

    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n\n[dependencies]\nshared_dep = { path = \"deps/shared_dep_v1\" }\n",
    )
    .expect("fixture setup should write tonic.toml");

    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    Shared.value()\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    fs::write(
        dep_v1.join("shared.tn"),
        "defmodule Shared do\n  def value() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write dependency v1 source");

    fs::write(
        dep_v2.join("shared.tn"),
        "defmodule Shared do\n  def value() do\n    2\n  end\nend\n",
    )
    .expect("fixture setup should write dependency v2 source");

    write_lockfile_with_path_dep(&fixture_root, &dep_v1);

    let first = run_with_cache_trace(&fixture_root);
    assert!(
        first.status.success(),
        "expected first run to succeed, got status {:?} and stderr: {}",
        first.status.code(),
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(
        String::from_utf8(first.stdout).expect("stdout should be utf8"),
        "1\n"
    );
    assert!(
        String::from_utf8(first.stderr)
            .expect("stderr should be utf8")
            .contains("cache-status miss"),
        "expected first run to be cache miss"
    );

    let second = run_with_cache_trace(&fixture_root);
    assert!(
        second.status.success(),
        "expected second run to succeed, got status {:?} and stderr: {}",
        second.status.code(),
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(
        String::from_utf8(second.stdout).expect("stdout should be utf8"),
        "1\n"
    );
    assert!(
        String::from_utf8(second.stderr)
            .expect("stderr should be utf8")
            .contains("cache-status hit"),
        "expected second run to be cache hit"
    );

    write_lockfile_with_path_dep(&fixture_root, &dep_v2);

    let third = run_with_cache_trace(&fixture_root);
    assert!(
        third.status.success(),
        "expected third run to succeed after lockfile change, got status {:?} and stderr: {}",
        third.status.code(),
        String::from_utf8_lossy(&third.stderr)
    );
    assert_eq!(
        String::from_utf8(third.stdout).expect("stdout should be utf8"),
        "2\n"
    );
    assert!(
        String::from_utf8(third.stderr)
            .expect("stderr should be utf8")
            .contains("cache-status miss"),
        "expected run after lockfile dependency change to be cache miss"
    );
}

fn write_lockfile_with_path_dep(project_root: &Path, dependency_path: &Path) {
    let canonical_path = dependency_path
        .canonicalize()
        .expect("dependency path should canonicalize");

    let lockfile = format!(
        "version = 1\n\n[path_deps.shared_dep]\npath = \"{}\"\n\n[git_deps]\n",
        canonical_path.to_string_lossy()
    );

    fs::write(project_root.join("tonic.lock"), lockfile)
        .expect("fixture setup should write tonic.lock");
}

fn run_with_cache_trace(fixture_root: &Path) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(fixture_root)
        .env("TONIC_DEBUG_CACHE", "1")
        .args(["run", "."])
        .output()
        .expect("run command should execute")
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
