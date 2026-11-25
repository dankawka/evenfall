//! Talking to KDE and to the evenfall daemon.

use std::env;
use std::path::PathBuf;
use std::process::Command;

use chrono::{Local, Timelike};

pub struct Output {
    pub name: String,
    pub current: Option<u8>,
}

pub fn now_minute() -> u16 {
    let now = Local::now();
    (now.hour() * 60 + now.minute()) as u16
}

pub fn clock() -> String {
    Local::now().format("%H:%M").to_string()
}

pub fn binary() -> PathBuf {
    if let Some(raw) = env::var_os("EVENFALL_BIN") {
        return PathBuf::from(raw);
    }
    if let Ok(exe) = env::current_exe() {
        // ui/target/debug/evenfall-ui -> ../../../evenfall (repo checkout)
        if let Some(root) = exe.parent().and_then(|p| p.parent()).and_then(|p| p.parent()) {
            let candidate = root.join("evenfall");
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    if let Some(home) = dirs::home_dir() {
        let candidate = home.join(".local/bin/evenfall");
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from("evenfall")
}

pub fn run_cli(args: &[&str]) -> Result<String, String> {
    let output = Command::new(binary())
        .args(args)
        .output()
        .map_err(|e| format!("cannot run evenfall: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            format!("evenfall {} failed", args.join(" "))
        } else {
            stderr
        })
    }
}

pub fn set_mode(mode: &str) -> Result<(), String> {
    run_cli(&["mode", mode]).map(|_| ())
}

pub fn ambient_sample() -> Option<(f32, i64)> {
    let path = dirs::state_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/state"))
        .join("evenfall")
        .join("ambient.json");
    let text = std::fs::read_to_string(path).ok()?;
    let lux: f32 = json_field(&text, "lux")?.parse().ok()?;
    let at = json_field(&text, "at")?;
    let at = at.trim_matches('"');
    let when = chrono::NaiveDateTime::parse_from_str(at, "%Y-%m-%dT%H:%M:%S").ok()?;
    let age = (chrono::Local::now().naive_local() - when).num_seconds().max(0);
    Some((lux, age))
}

fn json_field(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let start = text.find(&needle)? + needle.len();
    let rest = &text[start..];
    let rest = rest.trim_start().strip_prefix(':')?.trim_start();
    let end = rest.find([',', '}']).unwrap_or(rest.len());
    Some(rest[..end].trim().to_string())
}

pub fn identify(name: &str) -> Result<(), String> {
    run_cli(&["identify", name]).map(|_| ())
}

pub fn apply_brightness() -> Result<(), String> {
    run_cli(&["apply"]).map(|_| ())
}

pub fn unit_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("systemd/user/evenfall.service")
}

pub fn unit_installed() -> bool {
    unit_path().is_file()
}

pub fn install_daemon() -> Result<(), String> {
    run_cli(&["install"]).map(|_| ())
}

pub fn uninstall_daemon() -> Result<(), String> {
    run_cli(&["uninstall"]).map(|_| ())
}

pub fn service(action: &str) -> Result<(), String> {
    let output = Command::new("systemctl")
        .args(["--user", action, "evenfall.service"])
        .output()
        .map_err(|e| format!("systemctl: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn daemon_active() -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", "evenfall.service"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
        .unwrap_or(false)
}

pub fn pause_until() -> Option<String> {
    let dir = dirs::state_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/state"))
        .join("evenfall")
        .join("state.json");
    let text = std::fs::read_to_string(dir).ok()?;
    let key = "\"pause_until\"";
    let start = text.find(key)? + key.len();
    let rest = &text[start..];
    let start = rest.find('"')? + 1;
    let rest = &rest[start..];
    let end = rest.find('"')?;
    let raw = &rest[..end];
    let stamp = raw.get(11..16)?;
    // Only show a pause that is still running.
    let now = Local::now().format("%Y-%m-%dT%H:%M").to_string();
    if &raw[..16] > now.as_str() {
        Some(stamp.to_string())
    } else {
        None
    }
}

pub fn outputs() -> Vec<Output> {
    let raw = match Command::new("kscreen-doctor").arg("-o").output() {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).to_string()
        }
        _ => return Vec::new(),
    };
    let text = strip_ansi(&raw);
    let mut outputs: Vec<Output> = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Output: ") {
            let mut parts = rest.split_whitespace();
            parts.next(); // numeric id
            if let Some(name) = parts.next() {
                let name = name.to_uppercase();
                if name.contains('-') {
                    if let Some(slot) = outputs.iter_mut().find(|o| o.name == name) {
                        slot.current = None;
                    } else {
                        outputs.push(Output { name, current: None });
                    }
                }
            }
        } else if line.starts_with("Brightness control:") {
            if let Some(output) = outputs.last_mut() {
                output.current = parse_percent(line);
            }
        }
    }
    outputs
}

fn parse_percent(line: &str) -> Option<u8> {
    let start = line.find("set to ")? + "set to ".len();
    let rest = &line[start..];
    let end = rest.find('%')?;
    rest[..end].trim().parse().ok()
}

fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}
