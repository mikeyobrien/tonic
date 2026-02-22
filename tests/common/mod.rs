#![allow(dead_code)]

use std::path::PathBuf;

pub fn unique_fixture_root(test_name: &str) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    let path = std::env::temp_dir().join(format!(
        "tonic-{test_name}-{timestamp}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

pub fn unique_temp_dir(test_name: &str) -> PathBuf {
    unique_fixture_root(test_name)
}
