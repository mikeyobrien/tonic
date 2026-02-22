use crate::manifest::Dependencies;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const LOCKFILE_NAME: &str = "tonic.lock";
const DEPS_CACHE_DIR: &str = ".tonic/deps";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Lockfile {
    pub(crate) version: u32,
    pub(crate) path_deps: BTreeMap<String, PathDepLock>,
    pub(crate) git_deps: BTreeMap<String, GitDepLock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PathDepLock {
    pub(crate) path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GitDepLock {
    pub(crate) url: String,
    pub(crate) rev: String,
}

#[allow(dead_code)]
impl Lockfile {
    pub(crate) fn generate(
        dependencies: &Dependencies,
        _project_root: &Path,
    ) -> Result<Self, String> {
        let mut path_deps = BTreeMap::new();
        let mut git_deps = BTreeMap::new();

        for (name, path) in &dependencies.path {
            let canonical = path
                .canonicalize()
                .map_err(|e| format!("failed to canonicalize path dependency '{}': {}", name, e))?;
            path_deps.insert(
                name.clone(),
                PathDepLock {
                    path: canonical.to_string_lossy().to_string(),
                },
            );
        }

        for (name, git_dep) in &dependencies.git {
            git_deps.insert(
                name.clone(),
                GitDepLock {
                    url: git_dep.url.clone(),
                    rev: git_dep.rev.clone(),
                },
            );
        }

        Ok(Lockfile {
            version: 1,
            path_deps,
            git_deps,
        })
    }

    pub(crate) fn load(project_root: &Path) -> Result<Option<Self>, String> {
        let lockfile_path = project_root.join(LOCKFILE_NAME);
        if !lockfile_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&lockfile_path)
            .map_err(|e| format!("failed to read {}: {}", LOCKFILE_NAME, e))?;

        let lockfile: Lockfile =
            toml::from_str(&content).map_err(|e| format!("invalid {}: {}", LOCKFILE_NAME, e))?;

        Ok(Some(lockfile))
    }

    pub(crate) fn save(&self, project_root: &Path) -> Result<(), String> {
        let lockfile_path = project_root.join(LOCKFILE_NAME);
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("failed to serialize {}: {}", LOCKFILE_NAME, e))?;

        fs::write(&lockfile_path, content)
            .map_err(|e| format!("failed to write {}: {}", LOCKFILE_NAME, e))?;

        Ok(())
    }

    pub(crate) fn deps_dir(project_root: &Path) -> PathBuf {
        project_root.join(DEPS_CACHE_DIR)
    }

    pub(crate) fn resolved_git_dep(&self, name: &str) -> Option<&GitDepLock> {
        self.git_deps.get(name)
    }

    pub(crate) fn resolved_path_dep(&self, name: &str) -> Option<&PathDepLock> {
        self.path_deps.get(name)
    }
}

pub(crate) struct DependencyResolver;

impl DependencyResolver {
    /// Fetch all dependencies and generate lockfile
    pub(crate) fn sync(
        dependencies: &Dependencies,
        project_root: &Path,
    ) -> Result<Lockfile, String> {
        let lockfile = Lockfile::generate(dependencies, project_root)?;

        let deps_dir = Lockfile::deps_dir(project_root);
        fs::create_dir_all(&deps_dir)
            .map_err(|e| format!("failed to create deps directory: {}", e))?;

        // Fetch git dependencies
        for (name, git_lock) in &lockfile.git_deps {
            let cache_path = deps_dir.join(name);
            if cache_path.exists() {
                continue; // Already cached
            }

            Self::fetch_git_dep(&git_lock.url, &git_lock.rev, &cache_path)?;
        }

        lockfile.save(project_root)?;
        Ok(lockfile)
    }

    fn fetch_git_dep(url: &str, rev: &str, target_path: &Path) -> Result<(), String> {
        // Use git to fetch the specific revision
        let output = std::process::Command::new("git")
            .args([
                "clone",
                "--bare",
                "--no-checkout",
                url,
                target_path.to_str().unwrap(),
            ])
            .output()
            .map_err(|e| format!("failed to run git: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "git clone failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Checkout specific revision
        let output = std::process::Command::new("git")
            .current_dir(target_path)
            .args(["checkout", rev])
            .output()
            .map_err(|e| format!("failed to checkout rev {}: {}", rev, e))?;

        if !output.status.success() {
            return Err(format!(
                "git checkout failed for rev {}: {}",
                rev,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }

    /// Check if dependencies are available for offline use
    pub(crate) fn can_run_offline(lockfile: &Lockfile, project_root: &Path) -> bool {
        let deps_dir = Lockfile::deps_dir(project_root);

        for name in lockfile.git_deps.keys() {
            if !deps_dir.join(name).exists() {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Dependencies, GitDep};

    #[test]
    fn lockfile_generate_creates_correct_structure() {
        // Create a temp directory with a subdir to use as path dependency
        let temp_dir = std::env::temp_dir().join(format!("tonic-dep-test-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let subdir = temp_dir.join("local_lib");
        std::fs::create_dir_all(&subdir).unwrap();

        let mut deps = Dependencies::default();
        deps.path.insert("local_lib".to_string(), subdir.clone());
        deps.git.insert(
            "remote_lib".to_string(),
            GitDep {
                url: "https://github.com/example/lib.git".to_string(),
                rev: "abc123".to_string(),
            },
        );

        let lockfile = Lockfile::generate(&deps, &temp_dir).unwrap();

        assert_eq!(lockfile.version, 1);
        assert!(lockfile.path_deps.contains_key("local_lib"));
        assert!(lockfile.git_deps.contains_key("remote_lib"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn lockfile_save_and_load_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!("tonic-lock-test-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();

        let mut deps = Dependencies::default();
        deps.git.insert(
            "test_dep".to_string(),
            GitDep {
                url: "https://github.com/test/lib.git".to_string(),
                rev: "def456".to_string(),
            },
        );

        let lockfile = Lockfile::generate(&deps, &temp_dir).unwrap();
        lockfile.save(&temp_dir).unwrap();

        let loaded = Lockfile::load(&temp_dir).unwrap().unwrap();
        assert_eq!(loaded.version, 1);
        assert!(loaded.git_deps.contains_key("test_dep"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
