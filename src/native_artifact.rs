use crate::ir::IrProgram;
use crate::llvm_backend::LLVM_COMPATIBILITY_VERSION;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub(crate) const NATIVE_ARTIFACT_SCHEMA_VERSION: u32 = 1;
pub(crate) const NATIVE_BACKEND_LLVM: &str = "llvm";
pub(crate) const NATIVE_EMIT_EXECUTABLE: &str = "executable";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NativeArtifactManifest {
    pub(crate) schema_version: u32,
    pub(crate) backend: String,
    pub(crate) emit: String,
    pub(crate) target_triple: String,
    pub(crate) tonic_version: String,
    pub(crate) llvm_compatibility: String,
    pub(crate) source_hash: String,
    pub(crate) cache_key: String,
    pub(crate) artifacts: NativeArtifactFiles,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NativeArtifactFiles {
    pub(crate) ir: String,
    pub(crate) llvm_ir: String,
    pub(crate) object: String,
}

pub(crate) fn is_native_artifact_path(path: &str) -> bool {
    path.ends_with(".tnx.json")
}

pub(crate) fn host_target_triple() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

pub(crate) fn source_hash(source: &str) -> String {
    stable_hash(source)
}

pub(crate) fn native_artifact_cache_key(
    source_hash: &str,
    backend: &str,
    target_triple: &str,
    emit: &str,
) -> String {
    join_key_parts(&[
        source_hash,
        backend,
        target_triple,
        emit,
        env!("CARGO_PKG_VERSION"),
        LLVM_COMPATIBILITY_VERSION,
    ])
}

pub(crate) fn build_executable_manifest(
    source: &str,
    manifest_path: &Path,
    llvm_ir_path: &Path,
    object_path: &Path,
    ir_path: &Path,
) -> NativeArtifactManifest {
    let source_hash = source_hash(source);
    let target_triple = host_target_triple();
    let cache_key = native_artifact_cache_key(
        &source_hash,
        NATIVE_BACKEND_LLVM,
        &target_triple,
        NATIVE_EMIT_EXECUTABLE,
    );

    NativeArtifactManifest {
        schema_version: NATIVE_ARTIFACT_SCHEMA_VERSION,
        backend: NATIVE_BACKEND_LLVM.to_string(),
        emit: NATIVE_EMIT_EXECUTABLE.to_string(),
        target_triple,
        tonic_version: env!("CARGO_PKG_VERSION").to_string(),
        llvm_compatibility: LLVM_COMPATIBILITY_VERSION.to_string(),
        source_hash,
        cache_key,
        artifacts: NativeArtifactFiles {
            ir: relative_artifact_path(manifest_path, ir_path),
            llvm_ir: relative_artifact_path(manifest_path, llvm_ir_path),
            object: relative_artifact_path(manifest_path, object_path),
        },
    }
}

pub(crate) fn write_manifest(path: &Path, manifest: &NativeArtifactManifest) -> Result<(), String> {
    let serialized = serde_json::to_string(manifest)
        .map_err(|error| format!("failed to serialize native artifact manifest: {error}"))?;

    crate::cache::write_atomic(path, &serialized).map_err(|error| {
        format!(
            "failed to write native artifact manifest {}: {}",
            path.display(),
            error
        )
    })
}

pub(crate) fn load_manifest(path: &Path) -> Result<NativeArtifactManifest, String> {
    let raw = std::fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read native artifact manifest {}: {}",
            path.display(),
            error
        )
    })?;

    serde_json::from_str::<NativeArtifactManifest>(&raw).map_err(|error| {
        format!(
            "failed to parse native artifact manifest {}: {}",
            path.display(),
            error
        )
    })
}

pub(crate) fn validate_manifest_for_host(manifest: &NativeArtifactManifest) -> Result<(), String> {
    if manifest.schema_version != NATIVE_ARTIFACT_SCHEMA_VERSION {
        return Err(format!(
            "native artifact schema mismatch: expected {}, found {}",
            NATIVE_ARTIFACT_SCHEMA_VERSION, manifest.schema_version
        ));
    }

    if manifest.backend != NATIVE_BACKEND_LLVM {
        return Err(format!(
            "native artifact backend mismatch: expected {}, found {}",
            NATIVE_BACKEND_LLVM, manifest.backend
        ));
    }

    if manifest.emit != NATIVE_EMIT_EXECUTABLE {
        return Err(format!(
            "native artifact emit mismatch: expected {}, found {}",
            NATIVE_EMIT_EXECUTABLE, manifest.emit
        ));
    }

    let host_target = host_target_triple();
    if manifest.target_triple != host_target {
        return Err(format!(
            "native artifact target mismatch: artifact={} host={}",
            manifest.target_triple, host_target
        ));
    }

    if manifest.tonic_version != env!("CARGO_PKG_VERSION") {
        return Err(format!(
            "native artifact tonic version mismatch: artifact={} host={}",
            manifest.tonic_version,
            env!("CARGO_PKG_VERSION")
        ));
    }

    if manifest.llvm_compatibility != LLVM_COMPATIBILITY_VERSION {
        return Err(format!(
            "native artifact llvm compatibility mismatch: artifact={} host={}",
            manifest.llvm_compatibility, LLVM_COMPATIBILITY_VERSION
        ));
    }

    let expected_cache_key = native_artifact_cache_key(
        &manifest.source_hash,
        &manifest.backend,
        &manifest.target_triple,
        &manifest.emit,
    );
    if manifest.cache_key != expected_cache_key {
        return Err(format!(
            "native artifact cache key mismatch: expected {}, found {}",
            expected_cache_key, manifest.cache_key
        ));
    }

    Ok(())
}

pub(crate) fn load_ir_from_manifest(
    manifest_path: &Path,
    manifest: &NativeArtifactManifest,
) -> Result<IrProgram, String> {
    let ir_path = resolve_artifact_path(manifest_path, &manifest.artifacts.ir);
    let serialized = std::fs::read_to_string(&ir_path).map_err(|error| {
        format!(
            "failed to read native artifact ir {}: {}",
            ir_path.display(),
            error
        )
    })?;

    serde_json::from_str::<IrProgram>(&serialized).map_err(|error| {
        format!(
            "failed to parse native artifact ir {}: {}",
            ir_path.display(),
            error
        )
    })
}

fn relative_artifact_path(manifest_path: &Path, artifact_path: &Path) -> String {
    let Some(manifest_parent) = manifest_path.parent() else {
        return artifact_path.display().to_string();
    };

    artifact_path
        .strip_prefix(manifest_parent)
        .unwrap_or(artifact_path)
        .display()
        .to_string()
}

fn resolve_artifact_path(manifest_path: &Path, artifact: &str) -> PathBuf {
    let artifact_path = Path::new(artifact);
    if artifact_path.is_absolute() {
        artifact_path.to_path_buf()
    } else {
        manifest_path
            .parent()
            .map(|parent| parent.join(artifact_path))
            .unwrap_or_else(|| artifact_path.to_path_buf())
    }
}

fn join_key_parts(parts: &[&str]) -> String {
    let mut joined = String::new();

    for (idx, part) in parts.iter().enumerate() {
        if idx > 0 {
            joined.push('|');
        }

        joined.push_str(&part.len().to_string());
        joined.push(':');
        joined.push_str(part);
    }

    joined
}

fn stable_hash(content: &str) -> String {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS;
    for byte in content.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::{
        native_artifact_cache_key, source_hash, NATIVE_BACKEND_LLVM, NATIVE_EMIT_EXECUTABLE,
    };

    #[test]
    fn native_cache_key_changes_when_backend_changes() {
        let source_hash = source_hash("defmodule Demo do\nend\n");
        let target = "linux-x86_64";

        let llvm_key = native_artifact_cache_key(
            &source_hash,
            NATIVE_BACKEND_LLVM,
            target,
            NATIVE_EMIT_EXECUTABLE,
        );
        let interp_key = native_artifact_cache_key(&source_hash, "interp", target, "ir");

        assert_ne!(llvm_key, interp_key);
    }

    #[test]
    fn native_cache_key_changes_when_target_changes() {
        let source_hash = source_hash("defmodule Demo do\nend\n");

        let linux_key = native_artifact_cache_key(
            &source_hash,
            NATIVE_BACKEND_LLVM,
            "linux-x86_64",
            NATIVE_EMIT_EXECUTABLE,
        );
        let darwin_key = native_artifact_cache_key(
            &source_hash,
            NATIVE_BACKEND_LLVM,
            "darwin-aarch64",
            NATIVE_EMIT_EXECUTABLE,
        );

        assert_ne!(linux_key, darwin_key);
    }
}
