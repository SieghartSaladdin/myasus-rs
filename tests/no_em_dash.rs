//! No em dash (U+2014) anywhere in `src/`.
//!
//! It showed up in six strings people read: the window title, the display-test overlay, a
//! diagnosis summary, the panel line in System & Settings, a CLI line and a status message. It is
//! one of the most recognisable marks of machine-written text, so UI and CLI text uses a comma,
//! colon, period or parentheses instead.

mod common;

use std::collections::BTreeMap;

#[test]
fn no_em_dash_in_source_text() {
    let measured: BTreeMap<String, usize> = common::src_files()
        .into_iter()
        .map(|(file, text)| (file, text.lines().filter(|l| l.contains('\u{2014}')).count()))
        .collect();
    let problems = common::ratchet(
        "lines with an em dash",
        &measured,
        &[],
        0,
        "Replace the em dash with a comma (aside), a colon (explanation), a period (new sentence) \
         or parentheses. Text people read must not contain U+2014.",
    );
    assert!(problems.is_empty(), "\n{}\n", problems.join("\n\n"));
}
