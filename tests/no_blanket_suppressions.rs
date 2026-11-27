//! Compiler warnings must not be switched off in `src/`.
//!
//! A file-wide `#![allow(...)]` hid dead code in three files, so nothing ever told anyone that
//! whole types were unused. File-wide suppressions are banned; the item-level `#[allow(...)]`
//! lines that remain are frozen and can only go down.

mod common;

use std::collections::BTreeMap;

/// Files that still carry item-level `#[allow(...)]`, and how many. Only ever lower these.
const LEGACY: &[(&str, usize)] = &[
    ("src/models/hardware.rs", 2),
    ("src/platform/fallback_hw.rs", 2),
    ("src/platform/linux_oled.rs", 2),
    ("src/platform/mod.rs", 1),
];

fn count_lines(prefix: &str) -> BTreeMap<String, usize> {
    common::src_files()
        .into_iter()
        .map(|(file, text)| {
            let n = text.lines().filter(|l| l.trim_start().starts_with(prefix)).count();
            (file, n)
        })
        .collect()
}

#[test]
fn no_file_wide_allow() {
    let problems = common::ratchet(
        "`#![allow(..)]`",
        &count_lines("#![allow("),
        &[],
        0,
        "A file-wide `#![allow(..)]` hides every warning in the file, so unused or wrong code \
         never shows up. Fix the warning: delete the unused code or use it. For code only built \
         on another OS, put `#[cfg_attr(not(target_os = \"windows\"), allow(dead_code))]` on that \
         one item.",
    );
    assert!(problems.is_empty(), "\n{}\n", problems.join("\n\n"));
}

#[test]
fn no_new_item_level_allow() {
    let problems = common::ratchet(
        "`#[allow(..)]`",
        &count_lines("#[allow("),
        LEGACY,
        0,
        "`#[allow(..)]` silences a warning instead of fixing it. Delete the unused code or use \
         it; for code only built on one OS, scope it with `#[cfg_attr(<cfg>, allow(..))]`. Do \
         not raise LEGACY in tests/no_blanket_suppressions.rs.",
    );
    assert!(problems.is_empty(), "\n{}\n", problems.join("\n\n"));
}
