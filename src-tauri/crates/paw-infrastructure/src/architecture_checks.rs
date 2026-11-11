//! Architecture enforcement checks for E1-02 (paw-infrastructure unit tests).
//!
//! See `src-tauri/tests/architecture/dep_graph.rs` for the standalone
//! integration-test version; these are compiled into the crate so they run
//! with `cargo test --workspace`.

use std::fs;
use std::path::PathBuf;

#[allow(unused)]
fn package_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join(name))
        .expect("workspace root")
}

#[allow(unused)]
fn cargo_toml(name: &str) -> String {
    fs::read_to_string(package_root(name).join("Cargo.toml"))
        .unwrap_or_else(|e| panic!("{name}/Cargo.toml: {e}"))
}

#[allow(unused)]
fn dependencies_section(cargo_toml: &str) -> String {
    let mut in_deps = false;
    let mut out = String::new();
    for line in cargo_toml.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_deps = t == "[dependencies]";
            continue;
        }
        if in_deps {
            if t.is_empty() || t.starts_with('[') {
                break;
            }
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

#[allow(unused)]
fn dependency_names(cargo_toml: &str) -> Vec<String> {
    dependencies_section(cargo_toml)
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#') && !t.starts_with('[') && !t.contains("path")
        })
        .map(|l| {
            l.split(',')
                .next()
                .unwrap()
                .split('=')
                .next()
                .unwrap()
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// External crates paw-domain may depend on: narrowly selected pure utility
/// crates per `E1-rust-workspace-layout.md` §2. Anything outside this set is
/// a forbidden edge.
#[cfg(test)]
const DOMAIN_ALLOWED_EXTERNAL_CRATES: &[&str] = &["serde", "uuid"];

#[test]
fn paw_domain_external_dependencies_are_within_allowlist() {
    let deps = dependency_names(&cargo_toml("paw-domain"));
    let unexpected: Vec<&String> = deps
        .iter()
        .filter(|d| !DOMAIN_ALLOWED_EXTERNAL_CRATES.contains(&d.as_str()))
        .collect();
    assert!(
        unexpected.is_empty(),
        "paw-domain external dependencies must stay within {DOMAIN_ALLOWED_EXTERNAL_CRATES:?}; found: {unexpected:?}",
    );
}

#[test]
fn forbidden_crate_edges_are_rejected() {
    let by_name = |name: &str| -> Vec<String> { dependency_names(&cargo_toml(name)) };

    let domain = by_name("paw-domain");
    let application = by_name("paw-application");
    let infrastructure = by_name("paw-infrastructure");
    let desktop = by_name("paw-desktop");

    for dep in &application {
        assert!(
            *dep != "paw-infrastructure" && *dep != "paw-desktop",
            "paw-application may only depend on paw-domain; application={application:?}",
        );
    }

    for dep in &infrastructure {
        assert!(
            *dep != "paw-desktop",
            "paw-infrastructure may not depend on paw-desktop; infra={infrastructure:?}",
        );
    }

    for dep in &domain {
        assert!(
            dep != "paw-domain",
            "paw-domain must not depend on any PAW crate; domain={domain:?}",
        );
    }

    // paw-desktop is the composition root and the Tauri shell owner: besides
    // the two PAW crates it may only pull in the Tauri shell runtime and the
    // serialization primitives the command bridge needs (E1-rust-workspace
    // layout §4: map typed errors to stable Tauri-safe errors at the
    // boundary).
    for dep in &desktop {
        assert!(
            *dep == "paw-application"
                || *dep == "paw-infrastructure"
                || DESKTOP_ALLOWED_EXTERNAL_CRATES.contains(&dep.as_str()),
            "paw-desktop may only depend on application, infrastructure or the shell allowlist {DESKTOP_ALLOWED_EXTERNAL_CRATES:?}; desktop={desktop:?}",
        );
    }
}

/// External crates paw-desktop may depend on beyond the PAW crates: the
/// Tauri 2 shell runtime, serde (command-bridge serialization) and uuid
/// (per-command correlation ids for the audit trail).
/// Tauri is forbidden in every other crate (see
/// `only_paw_desktop_may_depend_on_tauri`).
#[cfg(test)]
const DESKTOP_ALLOWED_EXTERNAL_CRATES: &[&str] = &["tauri", "serde", "uuid", "rusqlite"];

#[test]
fn only_paw_desktop_may_depend_on_tauri() {
    for name in ["paw-domain", "paw-application", "paw-infrastructure"] {
        let deps = dependency_names(&cargo_toml(name));
        assert!(
            !deps.iter().any(|d| d == "tauri"),
            "only paw-desktop may depend on Tauri; {name} declared {deps:?}",
        );
    }
}

#[test]
fn paw_domain_must_not_import_sql_tauri_or_provider() {
    let mut found: Vec<(String, String)> = Vec::new();
    let src_root = package_root("paw-domain").join("src");
    let entries = fs::read_dir(&src_root).unwrap_or_else(|e| panic!("crates/paw-domain/src: {e}"));

    for entry in entries {
        let entry = entry.unwrap_or_else(|e| panic!("entry: {e}"));
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let src =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        for token in ["tauri::", "rusqlite::", "sqlx::", "tokio::", "provider::"] {
            if src.contains(token) {
                found.push((path.display().to_string(), token.to_string()));
            }
        }
    }

    assert!(
        found.is_empty(),
        "forbidden imports inside paw-domain: {}",
        found
            .iter()
            .map(|(p, t)| format!("{p} uses {t}"))
            .collect::<Vec<_>>()
            .join(", "),
    );
}
