//! Every `xrandr` call in `src/` must pass `--current`.
//!
//! Without it xrandr makes the driver re-probe every output, re-reading the panel's EDID. The
//! dimming guard did that about twice a second and the AMD laptop panel blanked. `--current`
//! reads the cached state instead. To change gamma, use `platform::x11_gamma` (RandR directly).

mod common;

use std::collections::BTreeMap;

const NEEDLE: &str = "Command::new(\"xrandr\")";

/// How many `xrandr` calls in `text` do not pass `--current` among their arguments.
fn calls_without_current(text: &str) -> usize {
    text.match_indices(NEEDLE)
        .filter(|(idx, _)| {
            // The call's arguments follow within a few lines; stop at the next command.
            let mut end = (idx + 400).min(text.len());
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            let window = &text[idx + NEEDLE.len()..end];
            let window = &window[..window.find("Command::new(").unwrap_or(window.len())];
            !window.contains("\"--current\"")
        })
        .count()
}

#[test]
fn every_xrandr_call_passes_current() {
    let measured: BTreeMap<String, usize> = common::src_files()
        .into_iter()
        .map(|(file, text)| (file, calls_without_current(&text)))
        .collect();
    let problems = common::ratchet(
        "`xrandr` calls without `--current`",
        &measured,
        &[],
        0,
        "Without `--current` xrandr makes the driver re-read the panel's EDID, which blanks the \
         screen. Add `\"--current\"` to its arguments, or use `crate::platform::x11_gamma` to \
         set gamma without spawning xrandr.",
    );
    assert!(problems.is_empty(), "\n{}\n", problems.join("\n\n"));
}
