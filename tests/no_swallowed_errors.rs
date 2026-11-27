//! Ratchet: no new `let _ = <call>` in `src/`.
//!
//! `let _ = fallible()` throws the error away, so a failed step (a gsettings write, a saved
//! preference, a Pixel Shift) leaves the feature silently broken and nobody finds out. Existing
//! occurrences are tolerated up to the counts below; new ones fail `cargo test`.

use std::fs;
use std::path::{Path, PathBuf};

/// Files that still swallow failures with `let _ =`, and how many times. Only ever lower these.
const LEGACY: &[(&str, usize)] = &[
    ("src/cli.rs", 1),
    ("src/platform/linux_hw.rs", 6),
    ("src/platform/linux_oled.rs", 14),
    ("src/platform/windows_oled.rs", 1),
    ("src/services/diagnostics.rs", 1),
    ("src/ui/app.rs", 2),
    ("src/ui/pages/oled_care.rs", 6),
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn swallowed(path: &Path) -> usize {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.trim_start().starts_with("let _ = "))
        .count()
}

#[test]
fn no_new_swallowed_errors() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    files.sort();

    let mut problems = Vec::new();
    for path in &files {
        let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        let found = swallowed(path);
        let allowed = LEGACY.iter().find(|(f, _)| *f == rel).map_or(0, |(_, n)| *n);
        if found > allowed {
            problems.push(format!(
                "{rel}: {found} `let _ =` (allowed {allowed}). `let _ = <call>` throws a failure \
                 away, so the feature stays silently broken. Propagate it with `?`, show it \
                 with `MyAsusState::record_action_result`, or for a best-effort step with no \
                 UI wrap it: `crate::services::failures::log_if_err(\"what failed\", result)`. \
                 Do not raise LEGACY in tests/no_swallowed_errors.rs."
            ));
        } else if found < allowed {
            problems.push(format!(
                "{rel}: down to {found} `let _ =` (was {allowed}). Lower its LEGACY count to \
                 {found} so the freed slot cannot be reused."
            ));
        }
    }
    for (file, _) in LEGACY {
        if !root.join(file).exists() {
            problems.push(format!("{file}: listed in LEGACY but no longer exists; remove it."));
        }
    }
    assert!(problems.is_empty(), "\n{}\n", problems.join("\n\n"));
}
