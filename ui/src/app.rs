use std::time::Duration;

use iced::Subscription;

use crate::config::{Config, Point};
use crate::curve;
use crate::system::{self, Output};

#[derive(Debug, Clone)]
pub enum Message {
    Poll,
    SelectMonitor(usize),
    ToggleEnabled(usize),
    PointPressed(u64),
    PointMoved(u64, u16, u8),
    PointReleased,
    AddPoint,
    DeletePoint(u64),
    TimeDraft(String),
    ValueDraft(String),
    SubmitTime,
    SubmitValue,
    ApplyNow,
    TogglePause,
    Refresh,
    InstallDaemon,
    UninstallDaemon,
    StartDaemon,
    StopDaemon,
}

pub struct App {
    pub cfg: Config,
    pub selected: usize,
    pub outputs: Vec<Output>,
    pub now: u16,
    pub paused: Option<String>,
    pub daemon: bool,
    pub installed: bool,
    pub status: String,
    pub ok: bool,
    pub selected_point: Option<u64>,
    pub time_draft: String,
    pub value_draft: String,
}

impl App {
    pub fn new() -> Self {
        let mut cfg = Config::load().unwrap_or_else(|err| {
            eprintln!("evenfall-ui: {err}");
            Config::default()
        });
        let outputs = system::outputs();
        for output in &outputs {
            cfg.add_monitor(&output.name);
        }
        let mut app = Self {
            cfg,
            selected: 0,
            outputs,
            now: system::now_minute(),
            paused: system::pause_until(),
            daemon: system::daemon_active(),
            installed: system::unit_installed(),
            status: "ready".to_string(),
            ok: true,
            selected_point: None,
            time_draft: String::new(),
            value_draft: String::new(),
        };
        app.refresh_hardware();
        if let Some(monitor) = app.cfg.monitors.first() {
            let points = monitor.points.clone();
            if let Some(point) = points.first() {
                app.select_point(Some(point.id));
            }
        }
        app
    }

    pub fn monitor(&self) -> Option<&crate::config::Monitor> {
        self.cfg.monitors.get(self.selected)
    }

    pub fn current_brightness(&self, name: &str) -> Option<u8> {
        self.outputs
            .iter()
            .find(|output| output.name == name)
            .and_then(|output| output.current)
    }

    pub fn connected(&self, name: &str) -> bool {
        self.outputs.iter().any(|output| output.name == name)
    }

    pub fn target(&self) -> Option<u8> {
        let monitor = self.monitor()?;
        let points = self.effective_points(monitor);
        curve::value_at(points, self.now)
    }

    pub fn effective_points<'a>(&'a self, monitor: &'a crate::config::Monitor) -> &'a [Point] {
        if monitor.points.is_empty() {
            &self.cfg.defaults
        } else {
            &monitor.points
        }
    }

    pub fn selected(&self) -> Option<Point> {
        let id = self.selected_point?;
        self.monitor()?.points.iter().copied().find(|p| p.id == id)
    }

    fn select_point(&mut self, id: Option<u64>) {
        self.selected_point = id;
        if let Some(point) = self.selected() {
            self.time_draft = curve::fmt(point.minute);
            self.value_draft = point.value.to_string();
        } else {
            self.time_draft.clear();
            self.value_draft.clear();
        }
    }

    fn refresh_hardware(&mut self) {
        self.outputs = system::outputs();
        for output in &self.outputs {
            self.cfg.add_monitor(&output.name);
        }
        self.now = system::now_minute();
        self.paused = system::pause_until();
        self.daemon = system::daemon_active();
        self.installed = system::unit_installed();
        if self.selected >= self.cfg.monitors.len() {
            self.selected = self.cfg.monitors.len().saturating_sub(1);
        }
    }

    fn service_action(&mut self, action: &str, message: &str) {
        match system::service(action) {
            Ok(()) => {
                self.ok = true;
                self.status = message.to_string();
            }
            Err(err) => {
                self.ok = false;
                self.status = err;
            }
        }
        self.refresh_hardware();
    }

    fn persist(&mut self, message: &str, apply: bool) {
        match self.cfg.save() {
            Ok(()) => {
                let text = if apply {
                    match system::apply_brightness() {
                        Ok(()) => format!("{message}, applied"),
                        Err(err) => format!("{message}, failed: {err}"),
                    }
                } else {
                    format!("{message}, daemon applies within {}s", self.cfg.tick)
                };
                self.ok = true;
                self.status = text;
            }
            Err(err) => {
                self.ok = false;
                self.status = err;
            }
        }
    }

    /// Move a keyframe. Refuses to land on top of another one, so dragging
    /// past a neighbour never silently deletes it.
    fn mutate_point(&mut self, id: u64, minute: u16, value: u8) -> bool {
        let Some(monitor) = self.cfg.monitors.get_mut(self.selected) else {
            return false;
        };
        if monitor.points.iter().any(|p| p.id != id && p.minute == minute) {
            return false;
        }
        if let Some(point) = monitor.points.iter_mut().find(|p| p.id == id) {
            point.minute = minute;
            point.value = value;
        } else {
            return false;
        }
        monitor.points.sort_by_key(|p| p.minute);
        if self.selected_point == Some(id) {
            self.time_draft = curve::fmt(minute);
            self.value_draft = value.to_string();
        }
        true
    }
}

pub fn boot() -> (App, iced::Task<Message>) {
    (App::new(), iced::Task::none())
}

pub fn subscription(_app: &App) -> Subscription<Message> {
    iced::time::every(Duration::from_secs(5)).map(|_| Message::Poll)
}

pub fn update(app: &mut App, message: Message) -> iced::Task<Message> {
    match message {
        Message::Poll | Message::Refresh => app.refresh_hardware(),

        Message::SelectMonitor(index) => {
            if index < app.cfg.monitors.len() {
                app.selected = index;
                app.selected_point = None;
                app.time_draft.clear();
                app.value_draft.clear();
            }
        }

        Message::ToggleEnabled(index) => {
            if let Some(monitor) = app.cfg.monitors.get_mut(index) {
                monitor.enabled = !monitor.enabled;
            }
            app.persist("saved", true);
        }

        Message::PointPressed(id) => {
            app.selected_point = Some(id);
            if let Some(point) = app.selected() {
                app.time_draft = curve::fmt(point.minute);
                app.value_draft = point.value.to_string();
            }
        }

        Message::PointMoved(id, minute, value) => {
            if app.monitor().map(|m| m.enabled).unwrap_or(false) {
                app.mutate_point(id, minute, value);
            }
        }

        Message::PointReleased => {
            app.persist("curve saved", true);
        }

        Message::AddPoint => {
            let minute = (app.now / 15) * 15;
            let value = app.target().unwrap_or_else(|| {
                app.monitor()
                    .and_then(|monitor| app.current_brightness(&monitor.name))
                    .unwrap_or(50)
            });
            let existing = app
                .monitor()
                .and_then(|monitor| monitor.points.iter().find(|p| p.minute == minute).copied());
            if let Some(point) = existing {
                app.selected_point = Some(point.id);
                app.time_draft = curve::fmt(point.minute);
                app.value_draft = point.value.to_string();
                app.ok = true;
                app.status = format!("a keyframe already sits at {}", curve::fmt(minute));
                return iced::Task::none();
            }
            let id = app.cfg.new_id();
            if let Some(monitor) = app.cfg.monitors.get_mut(app.selected) {
                monitor.points.push(Point { id, minute, value });
                monitor.points.sort_by_key(|p| p.minute);
            }
            app.selected_point = Some(id);
            app.time_draft = curve::fmt(minute);
            app.value_draft = value.to_string();
            app.persist("keyframe added", true);
        }

        Message::DeletePoint(id) => {
            if let Some(monitor) = app.cfg.monitors.get_mut(app.selected) {
                monitor.points.retain(|p| p.id != id);
            }
            if app.selected_point == Some(id) {
                app.selected_point = None;
                app.time_draft.clear();
                app.value_draft.clear();
            }
            app.persist("keyframe removed", true);
        }

        Message::TimeDraft(text) => app.time_draft = text,
        Message::ValueDraft(text) => app.value_draft = text,

        Message::SubmitTime => {
            let Some(id) = app.selected_point else { return iced::Task::none() };
            match curve::parse_time(&app.time_draft) {
                Some(minute) => {
                    let value = app.selected().map(|p| p.value).unwrap_or(50);
                    if app.mutate_point(id, minute, value) {
                        app.persist("keyframe saved", true);
                    } else {
                        app.ok = false;
                        app.status = format!("{} already has a keyframe", curve::fmt(minute));
                    }
                }
                None => {
                    app.ok = false;
                    app.status = "time must be HH:MM".to_string();
                }
            }
        }

        Message::SubmitValue => {
            let Some(id) = app.selected_point else { return iced::Task::none() };
            match app.value_draft.trim().parse::<i32>() {
                Ok(value) if (0..=100).contains(&value) => {
                    let minute = app.selected().map(|p| p.minute).unwrap_or(0);
                    app.mutate_point(id, minute, value as u8);
                    app.persist("keyframe saved", true);
                }
                _ => {
                    app.ok = false;
                    app.status = "brightness must be a number 0-100".to_string();
                }
            }
        }

        Message::ApplyNow => app.persist("applied now", true),

        Message::InstallDaemon => {
            match system::install_daemon() {
                Ok(()) => {
                    app.ok = true;
                    app.status = "daemon installed and started".to_string();
                }
                Err(err) => {
                    app.ok = false;
                    app.status = err;
                }
            }
            app.refresh_hardware();
        }

        Message::UninstallDaemon => {
            match system::uninstall_daemon() {
                Ok(()) => {
                    app.ok = true;
                    app.status = "daemon removed, curve kept in config".to_string();
                }
                Err(err) => {
                    app.ok = false;
                    app.status = err;
                }
            }
            app.refresh_hardware();
        }

        Message::StartDaemon => app.service_action("start", "daemon started"),

        Message::StopDaemon => app.service_action("stop", "daemon stopped"),

        Message::TogglePause => {
            let result = if app.paused.is_some() {
                system::run_cli(&["resume"]).map(|_| "resumed".to_string())
            } else {
                system::run_cli(&["pause", "1h"]).map(|_| "paused for 1h".to_string())
            };
            match result {
                Ok(text) => {
                    app.ok = true;
                    app.status = text;
                }
                Err(err) => {
                    app.ok = false;
                    app.status = err;
                }
            }
            app.paused = system::pause_until();
        }
    }
    iced::Task::none()
}

impl App {
    pub fn clock(&self) -> String {
        system::clock()
    }

    pub fn next_change(&self) -> Option<(String, u8)> {
        let monitor = self.monitor()?;
        let points = self.effective_points(monitor);
        curve::next_change(&points, self.now).map(|(minute, value)| (curve::fmt(minute), value))
    }
}
