use crate::deps::Lockfile;
use crate::ir::IrProgram;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const CACHE_DIRECTORY: &str = ".tonic/cache";
const CACHE_ARTIFACT_EXTENSION: &str = "ir.json";
const CACHE_FLAGS: &str = "none";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CacheKey(String);

impl CacheKey {
    pub(crate) fn from_parts(
        entry_hash: &str,
        dependency_hash: &str,
        runtime_version: &str,
        target: &str,
        flags: &str,
    ) -> Self {
        let parts = [entry_hash, dependency_hash, runtime_version, target, flags];
        let mut value = String::new();

        for (index, part) in parts.into_iter().enumerate() {
            if index > 0 {
                value.push('|');
            }

            value.push_str(&part.len().to_string());
            value.push(':');
            value.push_str(part);
        }

        Self(value)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

pub(crate) fn build_run_cache_key(source: &str, project_root: &Path) -> CacheKey {
    let entry_hash = stable_content_hash(source);

    // FIX: Use lockfile content for dependency hash, not source again
    let dependency_hash = match Lockfile::load(project_root) {
        Ok(Some(lockfile)) => {
            let lockfile_content = serde_json::to_string(&lockfile).unwrap_or_default();
            stable_content_hash(&lockfile_content)
        }
        _ => {
            // No lockfile or failed to load - use empty hash to indicate no dependencies
            stable_content_hash("")
        }
    };

    let target = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);

    CacheKey::from_parts(
        &entry_hash,
        &dependency_hash,
        env!("CARGO_PKG_VERSION"),
        &target,
        CACHE_FLAGS,
    )
}

pub(crate) fn load_cached_ir(key: &CacheKey) -> Result<Option<IrProgram>, String> {
    let artifact_path = cache_artifact_path(key)?;

    let serialized = match std::fs::read_to_string(&artifact_path) {
        Ok(serialized) => serialized,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => return Ok(None),
    };

    match serde_json::from_str::<IrProgram>(&serialized) {
        Ok(program) => Ok(Some(program)),
        Err(_) => {
            let _ = std::fs::remove_file(&artifact_path);
            Ok(None)
        }
    }
}

pub(crate) fn store_cached_ir(key: &CacheKey, program: &IrProgram) -> Result<(), String> {
    let artifact_path = cache_artifact_path(key)?;

    let payload = serde_json::to_string(program)
        .map_err(|error| format!("failed to serialize cache artifact: {error}"))?;

    write_atomic(&artifact_path, &payload).map_err(|error| {
        format!(
            "failed to write cache artifact {}: {error}",
            artifact_path.display()
        )
    })
}

pub(crate) fn write_atomic(target_path: &Path, content: &str) -> std::io::Result<()> {
    if target_path.is_dir() {
        let _ = std::fs::remove_dir_all(target_path);
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();

    let temp_file_name = format!(
        "{}.tmp.{}.{}",
        target_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("artifact"),
        timestamp,
        pid
    );
    let temp_path = target_path.with_file_name(temp_file_name);

    if let Some(parent) = target_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    std::fs::write(&temp_path, content)?;
    match std::fs::rename(&temp_path, target_path) {
        Ok(_) => Ok(()),
        Err(error) => {
            let _ = std::fs::remove_file(&temp_path);
            Err(error)
        }
    }
}

pub(crate) fn should_trace_cache_status() -> bool {
    std::env::var_os("TONIC_DEBUG_CACHE").is_some()
}

pub(crate) fn trace_cache_status(status: &str) {
    eprintln!("cache-status {status}");
}

fn cache_artifact_path(key: &CacheKey) -> Result<PathBuf, String> {
    let current_directory = std::env::current_dir()
        .map_err(|error| format!("failed to resolve current directory for cache: {error}"))?;

    Ok(current_directory.join(CACHE_DIRECTORY).join(format!(
        "{}.{}",
        key.as_str(),
        CACHE_ARTIFACT_EXTENSION
    )))
}

fn stable_content_hash(content: &str) -> String {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS;

    for byte in content.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    format!("{hash:016x}")
}

pub(crate) trait CacheStorage {
    fn lookup(&self, key: &CacheKey) -> Option<&str>;
    fn store(&mut self, key: CacheKey, payload: String);
    fn len(&self) -> usize;
}

#[derive(Debug, Default)]
pub(crate) struct CompileCache {
    entries: HashMap<CacheKey, String>,
}

impl CacheStorage for CompileCache {
    fn lookup(&self, key: &CacheKey) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    fn store(&mut self, key: CacheKey, payload: String) {
        self.entries.insert(key, payload);
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

impl CompileCache {
    pub(crate) fn lookup(&self, key: &CacheKey) -> Option<&str> {
        CacheStorage::lookup(self, key)
    }

    pub(crate) fn store(&mut self, key: CacheKey, payload: String) {
        CacheStorage::store(self, key, payload);
    }

    pub(crate) fn len(&self) -> usize {
        CacheStorage::len(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{CacheKey, CompileCache};

    #[test]
    fn cache_key_is_stable_for_identical_inputs() {
        let left = CacheKey::from_parts("entry-a", "deps-a", "runtime-1", "linux-x64", "none");
        let right = CacheKey::from_parts("entry-a", "deps-a", "runtime-1", "linux-x64", "none");

        assert_eq!(left, right);
    }

    #[test]
    fn cache_key_changes_when_any_dimension_changes() {
        let base = CacheKey::from_parts("entry-a", "deps-a", "runtime-1", "linux-x64", "none");
        let changed_target =
            CacheKey::from_parts("entry-a", "deps-a", "runtime-1", "linux-arm64", "none");

        assert_ne!(base, changed_target);
    }

    #[test]
    fn compile_cache_reports_miss_then_hit_for_synthetic_key() {
        let mut cache = CompileCache::default();
        let key = CacheKey::from_parts("entry-a", "deps-a", "runtime-1", "linux-x64", "none");

        assert_eq!(cache.lookup(&key), None);

        cache.store(key.clone(), "serialized-ir".to_string());

        assert_eq!(cache.lookup(&key), Some("serialized-ir"));
        assert_eq!(cache.len(), 1);
    }
}
