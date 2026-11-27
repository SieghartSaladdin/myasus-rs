use std::fs;
use std::path::Path;

pub fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn read_u32(path: impl AsRef<Path>) -> Option<u32> {
    read_trimmed(path)?.parse::<u32>().ok()
}

pub fn read_u64(path: impl AsRef<Path>) -> Option<u64> {
    read_trimmed(path)?.parse::<u64>().ok()
}

pub fn millidegrees_to_celsius(raw: i64) -> Option<f32> {
    let c = raw as f32 / 1000.0;
    if (-20.0..=140.0).contains(&c) {
        Some(c)
    } else {
        None
    }
}

pub fn compute_battery_health(full: u64, design: u64) -> Option<f32> {
    if design == 0 || full == 0 {
        return None;
    }
    Some(((full as f64 / design as f64) * 100.0).clamp(0.0, 120.0) as f32)
}

pub fn is_path_writable(path: impl AsRef<Path>) -> bool {
    fs::OpenOptions::new().write(true).open(path).is_ok()
}

/// Writes to a Linux sysfs attribute directly if permitted, otherwise falls back
/// to `pkexec tee` so Ubuntu / Linux Mint users get a native Polkit prompt.
#[cfg(target_os = "linux")]
pub fn write_sysfs_privileged(path: &Path, value: &str) -> Result<String, String> {
    if fs::write(path, value).is_ok() {
        return Ok(format!("Updated {} = {}", path.display(), value));
    }

    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("pkexec")
        .arg("tee")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "Cannot write {} directly and pkexec failed ({e}). Run packaging/install-ubuntu-mint.sh to install udev rules.",
                path.display()
            )
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(format!("{value}\n").as_bytes())
            .map_err(|e| format!("Failed writing value to pkexec stdin: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("pkexec wait error: {e}"))?;

    if output.status.success() {
        Ok(format!("Updated {} = {} (via Polkit)", path.display(), value))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!(
            "Permission denied writing {}: {}",
            path.display(),
            if stderr.is_empty() {
                "authorization dismissed or failed".to_string()
            } else {
                stderr
            }
        ))
    }
}

pub fn parse_os_release_pretty_name(contents: &str) -> Option<String> {
    for line in contents.lines() {
        if let Some(val) = line.strip_prefix("PRETTY_NAME=") {
            return Some(val.trim().trim_matches('"').to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_valid_hwmon_millidegrees() {
        assert_eq!(millidegrees_to_celsius(65_500), Some(65.5));
        assert_eq!(millidegrees_to_celsius(93_000), Some(93.0));
        assert_eq!(millidegrees_to_celsius(250_000), None);
    }

    #[test]
    fn computes_battery_health_percentage() {
        let health = compute_battery_health(42_000_000, 50_000_000).unwrap();
        assert!((health - 84.0).abs() < 0.01);
        assert_eq!(compute_battery_health(0, 50_000_000), None);
        assert_eq!(compute_battery_health(42_000_000, 0), None);
    }

    #[test]
    fn parses_linux_mint_os_release() {
        let sample = "NAME=\"Linux Mint\"\nPRETTY_NAME=\"Linux Mint 22.1\"\nID=linuxmint\n";
        assert_eq!(
            parse_os_release_pretty_name(sample),
            Some("Linux Mint 22.1".to_string())
        );
    }
}
