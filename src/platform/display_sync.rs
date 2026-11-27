//! Keeps the saved dimming level and the one on screen in step across processes.
//!
//! The login daemon and the app each run a dimming guard that re-applies whatever display.json
//! says when something reset the gamma ramp. Without a lock, a guard that had already loaded the
//! old level could put it back right after the user picked a new one (found by the exhaustive
//! check in scripts/model_dimming.py, see docs/dimming-flow.md). So "save the level and put it on
//! screen" and the guard's "load, compare, re-apply" each run under one lock file next to
//! display.json.

use std::fs::{self, File, TryLockError};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::platform::linux_oled::{apply_linux_display_pipeline, primary_xrandr_output};
use crate::platform::x11_gamma;
use crate::services::display_prefs::DisplayPrefs;

/// How long a UI action waits for the lock before going ahead without it, so a stuck holder can
/// never freeze the window.
const UI_WAIT: Duration = Duration::from_millis(500);

fn lock_path() -> Option<PathBuf> {
    DisplayPrefs::path().map(|p| p.with_file_name("display.lock"))
}

fn open(path: &Path) -> Option<File> {
    fs::create_dir_all(path.parent()?).ok()?;
    fs::OpenOptions::new().create(true).write(true).truncate(false).open(path).ok()
}

/// Runs `f` under the lock at `path`. Waits up to `wait` for another process or thread, then runs
/// `f` anyway; so does a lock that cannot be created or taken for any other reason.
fn with_lock_at<T>(path: &Path, wait: Duration, f: impl FnOnce() -> T) -> T {
    let Some(file) = open(path) else { return f() };
    let deadline = Instant::now() + wait;
    loop {
        match file.try_lock() {
            Ok(()) => break,
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(_) => return f(),
        }
    }
    f() // the lock is released when `file` is dropped
}

/// Runs `f` only if the lock is free right now: `None` when another process or thread holds it.
/// A lock that cannot be created or taken for any other reason does not block `f`.
fn try_with_lock_at<T>(path: &Path, f: impl FnOnce() -> T) -> Option<T> {
    let Some(file) = open(path) else { return Some(f()) };
    match file.try_lock() {
        Ok(()) | Err(TryLockError::Error(_)) => Some(f()),
        Err(TryLockError::WouldBlock) => None,
    }
}

fn save_then_apply_with(
    save: impl FnOnce() -> Result<(), String>,
    apply: impl FnOnce() -> Result<String, String>,
) -> Result<String, String> {
    let saved = save();
    let msg = apply()?;
    Ok(match saved {
        Ok(()) => msg,
        Err(e) => format!("{msg} (warning: not persisted: {e})"),
    })
}

/// Saves `prefs`, then puts them on screen, as one step under the lock.
///
/// Save first: the guards re-apply whatever is saved, so once the new level is on disk they can
/// only ever apply the new one.
pub fn save_then_apply(prefs: &DisplayPrefs) -> Result<String, String> {
    let run = || save_then_apply_with(|| prefs.save(), || apply_linux_display_pipeline(prefs));
    match lock_path() {
        Some(path) => with_lock_at(&path, UI_WAIT, run),
        None => run(),
    }
}

/// Re-applies Flicker-Free Dimming + Splendid gamma when something reset the CRTC gamma ramp.
///
/// On Cinnamon, `csd-color` reloads the default (identity) ramp about a second after every RandR
/// change: each Pixel Shift `--transform`, resume from suspend, unlock, DPMS wake or hotplug.
/// That wiped the dimming and left the panel at full brightness until something re-applied it.
/// Skips this tick when another process holds the lock (the next tick is a few seconds away).
/// Returns the apply message when it had to re-apply.
pub fn reassert_display_pipeline_if_reset() -> Option<String> {
    match lock_path() {
        Some(path) => try_with_lock_at(&path, reassert_unlocked).flatten(),
        None => reassert_unlocked(),
    }
}

fn reassert_unlocked() -> Option<String> {
    let prefs = DisplayPrefs::load();
    let neutral = prefs.flicker_free_dimming >= 100 && prefs.splendid_gamma_rgb() == (1.0, 1.0, 1.0);
    if neutral {
        // Nothing of ours to keep on the ramp; leave other tools (night light) alone.
        return None;
    }
    let output = primary_xrandr_output()?;
    let brightness = prefs.flicker_free_dimming.clamp(30, 100) as f32 / 100.0;
    if x11_gamma::output_gamma_matches(&output, prefs.splendid_gamma_rgb(), brightness)? {
        return None;
    }
    apply_linux_display_pipeline(&prefs).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::sync::mpsc;

    #[test]
    fn saves_before_it_applies_and_reports_a_failed_save() {
        let order = RefCell::new(Vec::new());
        let msg = save_then_apply_with(
            || {
                order.borrow_mut().push("save");
                Err("disk full".to_string())
            },
            || {
                order.borrow_mut().push("apply");
                Ok("applied".to_string())
            },
        );
        assert_eq!(*order.borrow(), ["save", "apply"]);
        assert_eq!(msg.unwrap(), "applied (warning: not persisted: disk full)");
    }

    #[test]
    fn an_apply_failure_is_returned() {
        let result = save_then_apply_with(|| Ok(()), || Err("no display".to_string()));
        assert_eq!(result, Err("no display".to_string()));
    }

    #[test]
    fn the_lock_excludes_waits_a_bounded_time_and_is_released() {
        let dir = std::env::temp_dir().join(format!("myasus-lock-test-{}", std::process::id()));
        let path = dir.join("display.lock");
        let (held_tx, held_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let holder_path = path.clone();
        let holder = std::thread::spawn(move || {
            with_lock_at(&holder_path, Duration::from_secs(1), || {
                held_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            })
        });
        held_rx.recv().unwrap();

        assert_eq!(try_with_lock_at(&path, || 1), None, "a held lock skips the guard's tick");
        let started = Instant::now();
        let ran = with_lock_at(&path, Duration::from_millis(40), || "ran anyway");
        assert_eq!(ran, "ran anyway", "a stuck holder must not freeze the caller");
        assert!(started.elapsed() >= Duration::from_millis(35), "it should have waited first");

        release_tx.send(()).unwrap();
        holder.join().unwrap();
        assert_eq!(try_with_lock_at(&path, || 7), Some(7), "the lock is free again");
        fs::remove_dir_all(&dir).ok();
    }
}
