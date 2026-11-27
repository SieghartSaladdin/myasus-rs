//! Ratchet: source files stay under 250 lines (AGENTS.md, single responsibility).
//!
//! A file over the limit is a grab bag, and the next change lands in it because it is already
//! there. The files below are frozen at their current length: they may shrink but never grow.
//! New files must stay within 250 lines.

mod common;

use std::collections::BTreeMap;

const LIMIT: usize = 250;

/// Files already over the limit, and their length. Only ever lower these; remove an entry once
/// the file is within the limit.
const LEGACY: &[(&str, usize)] = &[
    ("src/ui/pages/oled_care.rs", 976),
    ("src/platform/linux_oled.rs", 752),
    ("src/platform/linux_hw.rs", 631),
    ("src/platform/windows_hw.rs", 582),
    ("src/ui/app.rs", 579),
    ("src/ui/pages/home.rs", 395),
    ("src/services/diagnostics.rs", 391),
    ("src/ui/pages/diagnosis.rs", 373),
    ("src/services/display_prefs.rs", 332),
    ("src/ui/pages/hardware_power.rs", 303),
    ("src/cli.rs", 296),
    ("src/platform/windows_extra.rs", 295),
    ("src/platform/fallback_hw.rs", 267),
];

#[test]
fn source_files_do_not_grow_past_the_limit() {
    let measured: BTreeMap<String, usize> = common::src_files()
        .into_iter()
        .map(|(file, text)| (file, text.lines().count()))
        .collect();
    let problems = common::ratchet(
        "lines",
        &measured,
        LEGACY,
        LIMIT,
        "Put the new code in its own module next to this one (one domain per file, see AGENTS.md) \
         instead of growing this file. Do not raise LEGACY in tests/source_file_size.rs.",
    );
    assert!(problems.is_empty(), "\n{}\n", problems.join("\n\n"));
}
