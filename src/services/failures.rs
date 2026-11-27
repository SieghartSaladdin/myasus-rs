//! The one sanctioned way to drop a failure nobody can act on.
//!
//! `let _ = fallible()` throws the error away, so a failed step leaves the feature silently
//! broken. `tests/no_swallowed_errors.rs` rejects new ones; use this when a best-effort step
//! (a background daemon, a startup restore) has nowhere to show the error.

use std::fmt::Display;

/// Prints `step` and the error to stderr when `result` failed, and returns the success value.
///
/// Where the user can see a banner, report through `MyAsusState::record_action_result` instead.
pub fn log_if_err<T, E: Display>(step: &str, result: Result<T, E>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(err) => {
            eprintln!("myasus-rs: {step} failed: {err}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_value_on_success_and_drops_it_on_failure() {
        assert_eq!(log_if_err("ok step", Ok::<_, String>(7)), Some(7));
        assert_eq!(log_if_err("failing step", Err::<u8, _>("boom".to_string())), None);
    }
}
