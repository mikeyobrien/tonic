use super::{load_run_source, parse_manifest, Dependencies, ProjectManifest};
use crate::lexer::scan_tokens;
use crate::parser::parse_ast;
use std::path::{Path, PathBuf};

#[test]
fn embedded_stdlib_shadow_catalog_file_is_absent() {
    let shadow_catalog_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/stdlib_sources.rs");

    assert!(
        !shadow_catalog_path.exists(),
        "shadow stdlib catalog should stay absent: {}",
        shadow_catalog_path.display()
    );
}

#[test]
fn parse_manifest_requires_project_entry() {
    assert_eq!(
        parse_manifest("[project]\nname = \"demo\"\n", Path::new(".")),
        Err("invalid tonic.toml: missing required key project.entry".to_string())
    );
}

#[test]
fn parse_manifest_reads_project_entry() {
    assert_eq!(
        parse_manifest(
            "[project]\nname = \"demo\"\nentry = \"main.tn\"\n",
            Path::new("."),
        ),
        Ok(ProjectManifest {
            entry: PathBuf::from("main.tn"),
            dependencies: Dependencies::default(),
            package: None,
        })
    );
}

#[test]
fn parse_manifest_resolves_relative_path_dependencies_from_project_root() {
    let fixture_root = unique_fixture_root("manifest-relative-path-dependency");
    let project_root = fixture_root.join("app");
    let dep_root = fixture_root.join("shared_dep");

    std::fs::create_dir_all(&project_root)
        .expect("fixture setup should create project root directory");
    std::fs::create_dir_all(&dep_root).expect("fixture setup should create dependency directory");

    let manifest = parse_manifest(
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n\n[dependencies]\nshared_dep = { path = \"../shared_dep\" }\n",
        &project_root,
    )
    .expect("manifest should parse with relative path dependency");

    let resolved = manifest
        .dependencies
        .path
        .get("shared_dep")
        .expect("dependency should be present")
        .canonicalize()
        .expect("resolved dependency path should canonicalize");
    let expected = dep_root
        .canonicalize()
        .expect("expected dependency path should canonicalize");

    assert_eq!(resolved, expected);
}

#[test]
fn parse_manifest_rejects_dependency_without_path_or_git_source() {
    assert_eq!(
        parse_manifest(
            "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n\n[dependencies]\nbroken = { rev = \"abc123\" }\n",
            Path::new("."),
        ),
        Err(
            "invalid tonic.toml: dependency 'broken' must specify either a string 'path' or both string 'git' and 'rev'"
                .to_string()
        )
    );
}

#[test]
fn parse_manifest_rejects_path_dependency_with_non_string_path_value() {
    assert_eq!(
        parse_manifest(
            "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n\n[dependencies]\nbroken = { path = 42 }\n",
            Path::new("."),
        ),
        Err(
            "invalid tonic.toml: path dependency 'broken' has non-string 'path' value"
                .to_string()
        )
    );
}

#[test]
fn load_run_source_reads_manifest_entry_when_path_is_directory() {
    let fixture_root = unique_fixture_root("run-source-from-project-root");

    std::fs::create_dir_all(&fixture_root)
        .expect("fixture setup should create project root directory");
    std::fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    std::fs::create_dir_all(fixture_root.join("src"))
        .expect("fixture setup should create src directory");
    std::fs::write(
        fixture_root.join("src/main.tn"),
        "defmodule Demo do\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let source = load_run_source(
        fixture_root
            .to_str()
            .expect("fixture root should be valid utf-8"),
    )
    .expect("run source loading should succeed");

    assert_eq!(
        source,
        "defmodule Demo do\n  def run() do\n    1\n  end\nend\n"
    );
}

#[test]
fn load_run_source_includes_sibling_project_modules() {
    let fixture_root = unique_fixture_root("run-source-multi-module-project");

    std::fs::create_dir_all(fixture_root.join("src"))
        .expect("fixture setup should create src directory");
    std::fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    std::fs::write(
        fixture_root.join("src/main.tn"),
        "defmodule Demo do\n  def run() do\n    Math.helper()\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");
    std::fs::write(
        fixture_root.join("src/math.tn"),
        "defmodule Math do\n  def helper() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write sibling source");

    let source = load_run_source(
        fixture_root
            .to_str()
            .expect("fixture root should be valid utf-8"),
    )
    .expect("run source loading should succeed");

    let tokens = scan_tokens(&source).expect("loader source should tokenize");
    let ast = parse_ast(&tokens).expect("loader source should parse");

    assert_eq!(
        ast.modules
            .iter()
            .map(|module| module.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Demo", "Math"]
    );
}

// --- [package] metadata tests ---

#[test]
fn parse_manifest_reads_full_package_metadata() {
    use super::PackageMetadata;

    let source = "[project]\nentry = \"main.tn\"\n\n\
        [package]\n\
        name = \"my_lib\"\n\
        version = \"0.2.0\"\n\
        description = \"A sample library\"\n\
        license = \"MIT\"\n\
        repository = \"https://github.com/example/my_lib\"\n\
        authors = [\"Alice\", \"Bob\"]\n\
        keywords = [\"tonic\", \"library\"]\n";

    let manifest = parse_manifest(source, Path::new("."))
        .expect("manifest with full package section should parse");

    assert_eq!(
        manifest.package,
        Some(PackageMetadata {
            name: Some("my_lib".to_string()),
            version: Some("0.2.0".to_string()),
            description: Some("A sample library".to_string()),
            license: Some("MIT".to_string()),
            authors: vec!["Alice".to_string(), "Bob".to_string()],
            repository: Some("https://github.com/example/my_lib".to_string()),
            keywords: vec!["tonic".to_string(), "library".to_string()],
        })
    );
}

#[test]
fn parse_manifest_package_section_is_optional() {
    let source = "[project]\nentry = \"main.tn\"\n";
    let manifest =
        parse_manifest(source, Path::new(".")).expect("manifest without [package] should parse");
    assert_eq!(manifest.package, None);
}

#[test]
fn parse_manifest_partial_package_metadata_is_valid() {
    let source = "[project]\nentry = \"main.tn\"\n\n[package]\nname = \"core\"\n";
    let manifest = parse_manifest(source, Path::new("."))
        .expect("manifest with partial [package] should parse");
    let pkg = manifest
        .package
        .expect("[package] section should be present");
    assert_eq!(pkg.name, Some("core".to_string()));
    assert_eq!(pkg.version, None);
    assert_eq!(pkg.description, None);
    assert_eq!(pkg.authors, Vec::<String>::new());
}

// --- Registry dependency tests ---

#[test]
fn parse_manifest_reads_registry_dep_shorthand() {
    use super::RegistryDep;

    let source = "[project]\nentry = \"main.tn\"\n\n[dependencies]\njson = \"~> 1.0\"\n";
    let manifest = parse_manifest(source, Path::new("."))
        .expect("manifest with shorthand registry dep should parse");

    assert_eq!(
        manifest.dependencies.registry.get("json"),
        Some(&RegistryDep {
            version: "~> 1.0".to_string(),
            registry: None,
        })
    );
}

#[test]
fn parse_manifest_reads_registry_dep_table_form() {
    use super::RegistryDep;

    let source = "[project]\nentry = \"main.tn\"\n\n\
        [dependencies]\n\
        json = { version = \"^2.0\", registry = \"https://registry.example.com\" }\n";
    let manifest = parse_manifest(source, Path::new("."))
        .expect("manifest with table-form registry dep should parse");

    assert_eq!(
        manifest.dependencies.registry.get("json"),
        Some(&RegistryDep {
            version: "^2.0".to_string(),
            registry: Some("https://registry.example.com".to_string()),
        })
    );
}

#[test]
fn parse_manifest_reads_registry_dep_table_form_without_registry_override() {
    use super::RegistryDep;

    let source =
        "[project]\nentry = \"main.tn\"\n\n[dependencies]\nhttp = { version = \"~> 0.5\" }\n";
    let manifest = parse_manifest(source, Path::new("."))
        .expect("manifest with table-form registry dep (no registry override) should parse");

    assert_eq!(
        manifest.dependencies.registry.get("http"),
        Some(&RegistryDep {
            version: "~> 0.5".to_string(),
            registry: None,
        })
    );
}

#[test]
fn parse_manifest_rejects_package_name_non_string() {
    let source = "[project]\nentry = \"main.tn\"\n\n[package]\nname = 42\n";
    assert_eq!(
        parse_manifest(source, Path::new(".")),
        Err("invalid tonic.toml: package.name must be a string".to_string())
    );
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
