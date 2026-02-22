use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use std::fs;
use std::path::PathBuf;

fn unique_temp_dir(test_name: &str) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    let path = std::env::temp_dir().join(format!(
        "tonic-deps-lock-test-{test_name}-{timestamp}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("temp dir should be created");
    path
}

#[test]
fn deps_lock_generates_deterministic_lockfile_content() {
    let temp_dir = unique_temp_dir("deterministic-content");

    fs::create_dir_all(temp_dir.join("deps/path_a")).expect("fixture should create path_a");
    fs::create_dir_all(temp_dir.join("deps/path_z")).expect("fixture should create path_z");

    fs::write(
        temp_dir.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n\n[dependencies]\npath_a = { path = \"deps/path_a\" }\npath_z = { path = \"deps/path_z\" }\ngit_a = { git = \"https://example.com/a.git\", rev = \"1111111\" }\ngit_z = { git = \"https://example.com/z.git\", rev = \"2222222\" }\n",
    )
    .expect("fixture should write tonic.toml");

    let mut first = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));
    first.current_dir(&temp_dir);
    first
        .arg("deps")
        .arg("lock")
        .assert()
        .success()
        .stdout(contains("Lockfile generated: tonic.lock"));

    let first_lockfile =
        fs::read_to_string(temp_dir.join("tonic.lock")).expect("first lockfile should be readable");

    std::thread::sleep(std::time::Duration::from_secs(1));

    let mut second = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));
    second.current_dir(&temp_dir);
    second
        .arg("deps")
        .arg("lock")
        .assert()
        .success()
        .stdout(contains("Lockfile generated: tonic.lock"));

    let second_lockfile = fs::read_to_string(temp_dir.join("tonic.lock"))
        .expect("second lockfile should be readable");

    assert_eq!(
        first_lockfile, second_lockfile,
        "lockfile content should be deterministic across repeated generation"
    );

    assert!(
        !first_lockfile.contains("cached_at"),
        "deterministic lockfile format should not include volatile timestamp metadata"
    );
}
