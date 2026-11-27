//! Ratchet: no new `let _ = <call>` in `src/`.
//!
//! `let _ = fallible()` throws the error away, so a failed step (a gsettings write, a saved
//! preference, a Pixel Shift) leaves the feature silently broken and nobody finds out. Existing
//! occurrences are tolerated up to the counts below; new ones fail `cargo test`.

mod common;

use std::collections::BTreeMap;

/// Files that still swallow failures with `let _ =`, and how many times. Only ever lower these.
const LEGACY: &[(&str, usize)] = &[
    ("src/cli.rs", 1),
    ("src/platform/linux_hw.rs", 3),
    ("src/platform/linux_oled.rs", 14),
    ("src/platform/windows_oled.rs", 1),
    ("src/services/diagnostics.rs", 1),
    ("src/ui/app.rs", 2),
    ("src/ui/pages/oled_care.rs", 6),
];

#[test]
fn no_new_swallowed_errors() {
    let measured: BTreeMap<String, usize> = common::src_files()
        .into_iter()
        .map(|(file, text)| {
            let n = text.lines().filter(|l| l.trim_start().starts_with("let _ = ")).count();
            (file, n)
        })
        .collect();
    let problems = common::ratchet(
        "`let _ =`",
        &measured,
        LEGACY,
        0,
        "`let _ = <call>` throws a failure away, so the feature stays silently broken. Propagate \
         it with `?`, show it with `MyAsusState::record_action_result`, or for a best-effort step \
         with no UI wrap it: `crate::services::failures::log_if_err(\"what failed\", result)`. \
         Do not raise LEGACY in tests/no_swallowed_errors.rs.",
    );
    assert!(problems.is_empty(), "\n{}\n", problems.join("\n\n"));
}
