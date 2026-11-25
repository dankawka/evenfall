//! Window layout.

use iced::widget::{
    button, canvas, column, container, row, rule, scrollable, space, text, text_input, Space,
};
use iced::{Alignment, Element, Fill, Length, Theme};

use crate::app::{App, Message};
use crate::chart::Chart;
use crate::config::Point;
use crate::curve;
use crate::style::{self, bold, chip, ACCENT, BAD, BAD_BG, BORDER, MUTED, OK, OK_BG, TEXT, WARN, WARN_BG};

pub fn theme(_app: &App) -> Theme {
    style::theme()
}

pub fn view(app: &App) -> Element<'_, Message> {
    column![header(app), body(app), footer(app)]
        .width(Fill)
        .height(Fill)
        .into()
}

fn header(app: &App) -> Element<'_, Message> {
    let mut chips = row![].spacing(8).align_y(Alignment::Center);

    if let Some(until) = &app.paused {
        chips = chips.push(
            container(text(format!("paused until {until}")).size(11).color(WARN))
                .padding([4, 8])
                .style(chip(WARN_BG, WARN)),
        );
    }

    chips = chips.push(if app.daemon {
        container(text("daemon running").size(11).color(OK))
            .padding([4, 8])
            .style(chip(OK_BG, OK))
    } else {
        container(text("daemon stopped").size(11).color(BAD))
            .padding([4, 8])
            .style(chip(BAD_BG, BAD))
    });

    let actions = chips
        .push(
            button("Apply now")
                .style(button::primary)
                .on_press(Message::ApplyNow),
        )
        .push(
            button(if app.paused.is_some() { "Resume" } else { "Pause 1h" })
                .style(style::ghost)
                .on_press(Message::TogglePause),
        )
        .push(button("Refresh").style(style::quiet).on_press(Message::Refresh));

    container(
        row![
            text("evenfall").size(17).font(bold()).color(ACCENT),
            container(Space::new().width(1).height(18)).style(|_| container::Style {
                background: Some(BORDER.into()),
                ..container::Style::default()
            }),
            text("display brightness by time of day").size(13).color(MUTED),
            space::horizontal(),
            actions,
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .padding([0, 14]),
    )
    .style(style::header)
    .width(Fill)
    .height(52)
    .center_y(52)
    .into()
}

fn body(app: &App) -> Element<'_, Message> {
    if app.cfg.monitors.is_empty() {
        return container(text("Brak monitorów — sprawdź kscreen-doctor.").color(MUTED))
            .padding(24)
            .width(Fill)
            .into();
    }
    row![sidebar(app), main_panel(app)]
        .spacing(14)
        .padding(14)
        .height(Fill)
        .into()
}

fn sidebar(app: &App) -> Element<'_, Message> {
    let mut list = column![text("Monitors").size(11).color(MUTED)].spacing(6);

    for (index, monitor) in app.cfg.monitors.iter().enumerate() {
        let selected = index == app.selected;
        let current = app.current_brightness(&monitor.name);
        let target = curve::value_at(app.effective_points(monitor), app.now);
        let connected = app.connected(&monitor.name);

        let status = if !connected {
            "not connected".to_string()
        } else if let Some(current) = current {
            match target {
                Some(target) if target != current => format!("{current}%  →  {target}%"),
                _ => format!("{current}%"),
            }
        } else {
            "no DDC/CI".to_string()
        };

        let status_color = if !connected || (connected && current.is_none()) {
            MUTED
        } else {
            MUTED
        };

        let dot = if monitor.enabled { ACCENT } else { MUTED };

        list = list.push(
            row![
                button(
                    column![
                        row![
                            container(Space::new().width(6).height(6))
                                .style(chip(dot, dot)),
                            text(&monitor.name).size(14).font(bold()).color(TEXT),
                            space::horizontal(),
                            text(if monitor.enabled { "" } else { "off" })
                                .size(10)
                                .color(MUTED),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                        text(status).size(11).color(status_color),
                    ]
                    .spacing(3),
                )
                .style(style::monitor_row(selected))
                .on_press(Message::SelectMonitor(index))
                .padding([8, 10])
                .width(Fill),
                button(text("identify").size(10).color(MUTED))
                    .style(style::quiet)
                    .on_press(Message::Identify(monitor.name.clone()))
                    .padding([4, 6]),
            ]
            .spacing(2)
            .align_y(Alignment::Center),
        );
    }

    let (daemon_text, daemon_color) = if !app.installed {
        ("not installed", MUTED)
    } else if app.daemon {
        ("running", OK)
    } else {
        ("stopped", WARN)
    };

    let controls = if app.installed {
        row![
            button(if app.daemon { "Stop" } else { "Start" })
                .style(style::ghost)
                .on_press(if app.daemon {
                    Message::StopDaemon
                } else {
                    Message::StartDaemon
                }),
            button("Uninstall")
                .style(style::ghost)
                .on_press(Message::UninstallDaemon),
        ]
        .spacing(6)
    } else {
        row![button("Install daemon")
            .style(style::ghost)
            .on_press(Message::InstallDaemon)]
        .spacing(6)
    };

    container(
        column![
            list,
            space::vertical(),
            rule::horizontal(1),
            text("Schedule daemon").size(11).color(MUTED),
            text(daemon_text).size(11).color(daemon_color),
            controls,
            text("evenfall-ui 0.1").size(10).color(MUTED),
        ]
        .spacing(8),
    )
    .style(style::panel)
    .width(Length::Fixed(252.0))
    .height(Fill)
    .padding(10)
    .into()
}

fn main_panel(app: &App) -> Element<'_, Message> {
    if app.monitor().is_none() {
        return Space::new().into();
    }

    let content = column![summary(app), chart_card(app), points_card(app)].spacing(14);

    scrollable(content).height(Fill).into()
}

fn summary(app: &App) -> Element<'_, Message> {
    let monitor = app.monitor().unwrap();
    let current = app.current_brightness(&monitor.name);
    let target = app.target();

    let current_text = current
        .map(|value| format!("{value}%"))
        .unwrap_or_else(|| "—".to_string());
    let target_text = target
        .map(|value| format!("{value}%"))
        .unwrap_or_else(|| "—".to_string());
    let next_text = app
        .next_change()
        .map(|(time, value)| format!("{time} → {value}%"))
        .unwrap_or_else(|| "—".to_string());

    let toggle = button(if monitor.enabled { "disable" } else { "enable" })
        .style(style::ghost)
        .on_press(Message::ToggleEnabled(app.selected));

    container(
        row![
            stat("Now", current_text, TEXT),
            divider(),
            stat("Target", target_text, ACCENT),
            divider(),
            stat("Next", next_text, MUTED),
            space::horizontal(),
            column![
                text(&monitor.name).size(12).color(MUTED),
                toggle,
            ]
            .spacing(6)
            .align_x(Alignment::End),
        ]
        .spacing(18)
        .align_y(Alignment::Center)
        .padding([14, 18]),
    )
    .style(style::panel)
    .width(Fill)
    .into()
}

fn stat(label: &str, value: String, color: iced::Color) -> Element<'_, Message> {
    column![
        text(label.to_string()).size(10).color(MUTED),
        text(value).size(24).font(bold()).color(color),
    ]
    .spacing(2)
    .width(Length::Fixed(150.0))
    .into()
}

fn divider() -> Element<'static, Message> {
    container(Space::new().width(1).height(38))
        .style(|_| container::Style {
            background: Some(BORDER.into()),
            ..container::Style::default()
        })
        .into()
}

fn chart_card(app: &App) -> Element<'_, Message> {
    let monitor = app.monitor().unwrap();
    let points = app.effective_points(monitor);

    container(
        column![
            row![
                text("Day curve").size(13).font(bold()).color(TEXT),
                text("drag points, 15 min steps").size(11).color(MUTED),
                space::horizontal(),
                text(if monitor.points.is_empty() {
                    "defaults".to_string()
                } else {
                    format!("{} pts", monitor.points.len())
                })
                .size(11)
                .color(MUTED),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            canvas(Chart {
                points,
                now: app.now,
                enabled: monitor.enabled,
            })
            .width(Fill)
            .height(Length::Fixed(300.0)),
        ]
        .spacing(10),
    )
    .style(style::panel)
    .padding(14)
    .width(Fill)
    .into()
}

fn points_card(app: &App) -> Element<'_, Message> {
    let monitor = app.monitor().unwrap();
    let mut list = column![].spacing(4);

    if monitor.points.is_empty() {
        list = list.push(text("This monitor follows the defaults curve.").size(12).color(MUTED));
    }

    for point in &monitor.points {
        list = list.push(point_row(app, point));
    }

    let mut card = column![
        row![
            text("Keyframes").size(13).font(bold()).color(TEXT),
            space::horizontal(),
            button("+ add keyframe")
                .style(style::ghost)
                .on_press(Message::AddPoint),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        list,
    ]
    .spacing(10);

    if app.selected().is_some() {
        card = card.push(editor(app));
    }

    container(card).style(style::panel).padding(14).width(Fill).into()
}

fn point_row<'a>(app: &'a App, point: &'a Point) -> Element<'a, Message> {
    let selected = app.selected_point == Some(point.id);
    let value = app
        .monitor()
        .and_then(|monitor| monitor.points.iter().find(|p| p.id == point.id))
        .map(|p| p.value)
        .unwrap_or(point.value);

    let body = row![
        text(curve::fmt(point.minute)).size(13).font(bold()).color(if selected { ACCENT } else { TEXT }),
        text(format!("{value}%")).size(12).color(MUTED),
        space::horizontal(),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    row![
        button(body)
            .style(style::monitor_row(selected))
            .on_press(Message::PointPressed(point.id))
            .padding([7, 10])
            .width(Fill),
        button(text("×").size(15).color(MUTED))
            .style(style::danger)
            .on_press(Message::DeletePoint(point.id))
            .padding([4, 8]),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}

fn editor(app: &App) -> Element<'_, Message> {
    container(
        row![
            text("point").size(11).color(MUTED),
            text_input("HH:MM", &app.time_draft)
                .style(style::input)
                .on_input(Message::TimeDraft)
                .on_submit(Message::SubmitTime)
                .padding(6)
                .width(Length::Fixed(84.0)),
            text_input("0-100", &app.value_draft)
                .style(style::input)
                .on_input(Message::ValueDraft)
                .on_submit(Message::SubmitValue)
                .padding(6)
                .width(Length::Fixed(72.0)),
            text("%").size(12).color(MUTED),
            space::horizontal(),
            text("Enter saves").size(10).color(MUTED),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .style(style::inset)
    .padding(8)
    .width(Fill)
    .into()
}

fn footer(app: &App) -> Element<'_, Message> {
    let color = if app.ok { OK } else { BAD };
    let path = crate::config::path();
    let home = dirs::home_dir().unwrap_or_default().to_string_lossy().to_string();
    let shown = path
        .to_string_lossy()
        .replace(home.as_str(), "~");

    column![
        rule::horizontal(1),
        container(
            row![
                text(&app.status).size(12).color(color),
                space::horizontal(),
                text(shown).size(10).color(MUTED),
                text(format!("{}", app.clock())).size(11).color(MUTED),
            ]
            .spacing(14)
            .align_y(Alignment::Center)
            .padding([8, 14]),
        )
        .style(style::header)
        .width(Fill),
    ]
    .into()
}
