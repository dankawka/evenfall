//! Reading and writing the evenfall TOML config without pulling a TOML crate.

use std::fs;
use std::path::PathBuf;

use crate::curve;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub id: u64,
    pub minute: u16,
    pub value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LuxPoint {
    pub lux: u32,
    pub value: u8,
}

#[derive(Debug, Clone)]
pub struct Monitor {
    pub name: String,
    pub enabled: bool,
    pub points: Vec<Point>,
    pub ambient: Vec<LuxPoint>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub tick: u64,
    pub reassert_minutes: u64,
    pub defaults: Vec<Point>,
    pub monitors: Vec<Monitor>,
    pub mode: String,
    pub ambient: Vec<LuxPoint>,
    next_id: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tick: 60,
            reassert_minutes: 0,
            defaults: Vec::new(),
            monitors: Vec::new(),
            mode: "schedule".to_string(),
            ambient: Vec::new(),
            next_id: 1,
        }
    }
}

pub fn path() -> PathBuf {
    if let Some(raw) = std::env::var_os("EVENFALL_CONFIG") {
        return PathBuf::from(raw);
    }
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("evenfall").join("config.toml")
}

impl Config {
    pub fn new_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn monitor_index(&self, name: &str) -> Option<usize> {
        self.monitors.iter().position(|m| m.name == name)
    }

    pub fn add_monitor(&mut self, name: &str) {
        if self.monitor_index(name).is_some() {
            return;
        }
        self.monitors.push(Monitor {
            name: name.to_string(),
            enabled: true,
            points: Vec::new(),
            ambient: Vec::new(),
        });
    }

    pub fn load() -> Result<Self, String> {
        let file = path();
        match fs::read_to_string(&file) {
            Ok(text) => Ok(Self::parse(&text)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(format!("{}: {err}", file.display())),
        }
    }

    pub fn parse(text: &str) -> Self {
        let mut config = Self::default();
        let mut section: Section = Section::Root;

        for raw in text.lines() {
            let line = strip_comment(raw).trim().to_string();
            if line.is_empty() {
                continue;
            }

            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                let name = name.trim();
                if name == "defaults" {
                    section = Section::Defaults;
                } else if name == "ambient" {
                    section = Section::Ambient(None);
                } else if let Some(monitor) = name.strip_prefix("monitors.") {
                    let monitor = monitor.trim().trim_matches('"').to_string();
                    config.add_monitor(&monitor);
                    let index = config.monitor_index(&monitor).unwrap_or(0);
                    section = Section::Monitor(index);
                } else if let Some(monitor) = name.strip_prefix("ambient.") {
                    let monitor = monitor.trim().trim_matches('"').to_string();
                    config.add_monitor(&monitor);
                    let index = config.monitor_index(&monitor).unwrap_or(0);
                    section = Section::Ambient(Some(index));
                } else {
                    section = Section::Other;
                }
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim().trim_matches('"');
            let value = value.trim();

            match section {
                Section::Root => match key {
                    "tick" => config.tick = value.parse().unwrap_or(60).max(10),
                    "reassert_minutes" => {
                        config.reassert_minutes = value.parse().unwrap_or(0)
                    }
                    "mode" => {
                        let mode = value.trim().trim_matches('"').to_lowercase();
                        if mode == "ambient" || mode == "schedule" {
                            config.mode = mode;
                        }
                    }
                    _ => {}
                },
                Section::Defaults => {
                    if let Some(point) = parse_point(&mut config, key, value) {
                        config.defaults.push(point);
                    }
                }
                Section::Monitor(index) => {
                    if key == "enabled" {
                        config.monitors[index].enabled = value != "false";
                        continue;
                    }
                    if let Some(point) = parse_point(&mut config, key, value) {
                        config.monitors[index].points.push(point);
                    }
                }
                Section::Ambient(index) => {
                    if let Some(point) = parse_lux(key, value) {
                        match index {
                            Some(index) => config.monitors[index].ambient.push(point),
                            None => config.ambient.push(point),
                        }
                    }
                }
                Section::Other => {}
            }
        }

        config.defaults.sort_by_key(|p| p.minute);
        config.ambient.sort_by_key(|p| p.lux);
        for monitor in &mut config.monitors {
            monitor.points.sort_by_key(|p| p.minute);
            monitor.ambient.sort_by_key(|p| p.lux);
        }
        config
    }

    pub fn save(&self) -> Result<(), String> {
        let file = path();
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(&file, self.render()).map_err(|e| format!("{}: {e}", file.display()))
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("# evenfall — display brightness by time of day.\n");
        out.push_str("# Values are 0-100 (%), interpolated linearly between keyframes.\n");
        out.push_str("# Keyframes on both sides of midnight close the loop automatically.\n");
        out.push_str("# Written by evenfall-ui; the daemon reloads it on every tick.\n\n");
        out.push_str(&format!("tick = {}\n", self.tick));
        out.push_str(&format!("reassert_minutes = {}\n", self.reassert_minutes));
        out.push_str(&format!("mode = \"{}\"\n", self.mode));

        if !self.ambient.is_empty() {
            out.push_str("\n# Ambient mode: brightness from illuminance instead of the clock.\n[ambient]\n");
            for point in &self.ambient {
                out.push_str(&format!("\"{}\" = {}\n", point.lux, point.value));
            }
        }

        out.push_str("\n# Shared curve for monitors without their own section.\n[defaults]\n");
        for point in &self.defaults {
            out.push_str(&point_line(point));
        }

        for monitor in &self.monitors {
            out.push_str(&format!(
                "\n[monitors.\"{}\"]\n",
                monitor.name
            ));
            if !monitor.enabled {
                out.push_str("enabled = false\n");
            }
            if monitor.points.is_empty() {
                out.push_str("# no keyframes: the [defaults] curve applies\n");
            }
            for point in &monitor.points {
                out.push_str(&point_line(point));
            }
            if !monitor.ambient.is_empty() {
                out.push_str(&format!("\n[ambient.\"{}\"]\n", monitor.name));
                for point in &monitor.ambient {
                    out.push_str(&format!("\"{}\" = {}\n", point.lux, point.value));
                }
            }
        }
        out
    }
}

enum Section {
    Root,
    Defaults,
    Monitor(usize),
    Ambient(Option<usize>),
    Other,
}

fn parse_lux(key: &str, value: &str) -> Option<LuxPoint> {
    let lux: f32 = key.trim().trim_matches('"').parse().ok()?;
    if lux < 0.0 {
        return None;
    }
    let value: i32 = value.trim().parse().ok()?;
    Some(LuxPoint {
        lux: lux as u32,
        value: value.clamp(0, 100) as u8,
    })
}

fn point_line(point: &Point) -> String {
    format!("\"{}\" = {}\n", curve::fmt(point.minute), point.value)
}

fn parse_point(config: &mut Config, key: &str, value: &str) -> Option<Point> {
    let minute = match key.split_once(':') {
        Some(_) => curve::parse_time(key)?,
        None => return None,
    };
    let value: i32 = value.trim().trim_matches('"').parse().ok()?;
    Some(Point {
        id: config.new_id(),
        minute,
        value: value.clamp(0, 100) as u8,
    })
}

fn strip_comment(line: &str) -> &str {
    let mut in_quote = false;
    for (index, ch) in line.char_indices() {
        match ch {
            '"' => in_quote = !in_quote,
            '#' if !in_quote => return &line[..index],
            _ => {}
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambient_sections_do_not_leak_into_monitors() {
        let text = r#"
mode = "ambient"

[ambient]
"5" = 10
"300" = 55

[ambient."DP-2"]
"5" = 5
"500" = 40

[monitors."DP-3"]
"10:00" = 80

[monitors."DP-2"]
"10:00" = 50
"16:00" = 10
"#;
        let cfg = Config::parse(text);
        assert_eq!(cfg.mode, "ambient");
        assert_eq!(cfg.ambient.len(), 2);

        let dp3 = cfg.monitors.iter().find(|m| m.name == "DP-3").unwrap();
        assert_eq!(dp3.points.len(), 1);
        assert!(dp3.ambient.is_empty());

        let dp2 = cfg.monitors.iter().find(|m| m.name == "DP-2").unwrap();
        assert_eq!(dp2.points.len(), 2);
        assert_eq!(dp2.ambient.len(), 2);

        // Rendering must stay valid TOML: no repeated key inside one table.
        let rendered = cfg.render();
        assert_eq!(rendered.matches("\"10:00\" = 80").count(), 1);
        assert_eq!(rendered.matches("[ambient.").count(), 1);
    }

    #[test]
    fn render_roundtrip() {
        let mut cfg = Config::default();
        cfg.defaults.push(Point { id: 1, minute: 360, value: 50 });
        cfg.add_monitor("DP-3");
        let id = cfg.new_id();
        cfg.monitors[0].points.push(Point { id, minute: 600, value: 80 });
        cfg.monitors[0].enabled = false;

        let rendered = cfg.render();
        // The daemon parses this with tomllib, so the shape must stay valid.
        assert!(rendered.contains("[monitors.\"DP-3\"]"));
        assert!(rendered.contains("enabled = false"));
        assert!(rendered.contains("\"10:00\" = 80"));
    }
}
