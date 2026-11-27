//! Shared by the source-scanning tests. Each test file declares `mod common;`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Every `src/**/*.rs` file as `(path relative to the repo root with '/', contents)`, sorted.
pub fn src_files() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    collect(&root.join("src"), &mut paths);
    paths.sort();
    paths
        .iter()
        .map(|path| {
            let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            (rel, fs::read_to_string(path).unwrap_or_default())
        })
        .collect()
}

/// Compares a per-file measurement with a frozen `legacy` table and returns what is wrong.
///
/// A file may measure up to `limit`, or up to its `legacy` entry if it has one. Entries can only
/// go down: when a legacy file improves, the entry must be lowered (or removed once the file is
/// within `limit`), so the freed room cannot be reused. `fix` tells the reader what to do instead.
pub fn ratchet(
    unit: &str,
    measured: &BTreeMap<String, usize>,
    legacy: &[(&str, usize)],
    limit: usize,
    fix: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    for (file, &found) in measured {
        let listed = legacy.iter().find(|(f, _)| f == file).map(|(_, n)| *n);
        let allowed = listed.unwrap_or(limit);
        if found > allowed {
            problems.push(format!("{file}: {found} {unit} (allowed {allowed}). {fix}"));
        } else if listed.is_some() && found <= limit {
            problems.push(format!(
                "{file}: now {found} {unit}, within the limit of {limit}. Remove it from LEGACY."
            ));
        } else if listed.is_some() && found < allowed {
            problems.push(format!(
                "{file}: down to {found} {unit} (was {allowed}). Lower its LEGACY entry to {found} \
                 so the freed room cannot be reused."
            ));
        }
    }
    for (file, _) in legacy {
        if !measured.contains_key(*file) {
            problems.push(format!("{file}: listed in LEGACY but no longer exists. Remove it."));
        }
    }
    problems
}
