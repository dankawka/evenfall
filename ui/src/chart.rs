//! The day-curve chart: draws the schedule and lets you drag keyframes.

use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

use crate::config::Point as Keyframe;
use crate::curve;
use crate::style::{ACCENT, ACCENT_SOFT, BG, BORDER, GOLD, MUTED, TEXT};
use crate::Message;

const PAD_LEFT: f32 = 40.0;
const PAD_RIGHT: f32 = 18.0;
const PAD_TOP: f32 = 18.0;
const PAD_BOTTOM: f32 = 26.0;
const HANDLE_RADIUS: f32 = 5.0;

pub struct Chart<'a> {
    pub points: &'a [Keyframe],
    pub now: u16,
    pub enabled: bool,
}

#[derive(Default)]
pub struct State {
    dragging: Option<u64>,
}

impl<'a> Chart<'a> {
    fn plot(&self, bounds: Rectangle) -> Rectangle {
        Rectangle {
            x: PAD_LEFT,
            y: PAD_TOP,
            width: (bounds.width - PAD_LEFT - PAD_RIGHT).max(10.0),
            height: (bounds.height - PAD_TOP - PAD_BOTTOM).max(10.0),
        }
    }

    fn x_of(&self, plot: Rectangle, minute: u16) -> f32 {
        plot.x + plot.width * (minute as f32 / 1440.0)
    }

    fn y_of(&self, plot: Rectangle, value: u8) -> f32 {
        plot.y + plot.height * (1.0 - value as f32 / 100.0)
    }

    fn map(&self, position: Point, bounds: Rectangle) -> (u16, u8) {
        let plot = self.plot(bounds);
        let tx = ((position.x - plot.x) / plot.width).clamp(0.0, 1.0);
        let ty = ((position.y - plot.y) / plot.height).clamp(0.0, 1.0);
        let minute = (tx * 1440.0).round() as i32;
        let minute = ((minute + 7) / 15 * 15).clamp(0, 1425) as u16;
        let value = (100.0 * (1.0 - ty)).round().clamp(0.0, 100.0) as u8;
        (minute, value)
    }

    fn hit(&self, position: Point, bounds: Rectangle) -> Option<Keyframe> {
        let plot = self.plot(bounds);
        self.points
            .iter()
            .map(|point| {
                let center = Point::new(self.x_of(plot, point.minute), self.y_of(plot, point.value));
                let distance = ((center.x - position.x).powi(2) + (center.y - position.y).powi(2)).sqrt();
                (distance, point)
            })
            .filter(|(distance, _)| *distance <= 16.0)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, point)| *point)
    }
}

impl<'a> canvas::Program<Message> for Chart<'a> {
    type State = State;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if !self.enabled {
                    return None;
                }
                if let Some(position) = cursor.position_in(bounds) {
                    if let Some(point) = self.hit(position, bounds) {
                        state.dragging = Some(point.id);
                        return Some(canvas::Action::publish(Message::PointPressed(point.id)));
                    }
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let (Some(id), Some(position)) = (state.dragging, cursor.position_in(bounds)) {
                    let (minute, value) = self.map(position, bounds);
                    return Some(canvas::Action::publish(Message::PointMoved(id, minute, value)));
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.dragging.take().is_some() {
                    return Some(canvas::Action::publish(Message::PointReleased));
                }
            }
            _ => {}
        }
        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let plot = self.plot(bounds);

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), BG);

        // Horizontal grid every 25 %.
        for step in 0..=4 {
            let value = (step * 25) as u8;
            let y = self.y_of(plot, value);
            frame.stroke(
                &Path::line(Point::new(plot.x, y), Point::new(plot.x + plot.width, y)),
                Stroke::default()
                    .with_color(Color { a: 0.10, ..BORDER })
                    .with_width(1.0),
            );
            frame.fill_text(Text {
                content: format!("{value}"),
                position: Point::new(plot.x - 8.0, y),
                color: MUTED,
                size: 10.0.into(),
                align_x: iced::alignment::Horizontal::Right.into(),
                align_y: iced::alignment::Vertical::Center,
                ..Text::default()
            });
        }

        // Vertical grid every 3 hours, labels every 6.
        for hour in (0..=24).step_by(3) {
            let x = plot.x + plot.width * (hour as f32 / 24.0);
            frame.stroke(
                &Path::line(Point::new(x, plot.y), Point::new(x, plot.y + plot.height)),
                Stroke::default()
                    .with_color(Color { a: 0.08, ..BORDER })
                    .with_width(1.0),
            );
            if hour % 6 == 0 {
                frame.fill_text(Text {
                    content: format!("{hour:02}"),
                    position: Point::new(x, plot.y + plot.height + 6.0),
                    color: MUTED,
                    size: 10.0.into(),
                    align_x: iced::alignment::Horizontal::Center.into(),
                    align_y: iced::alignment::Vertical::Top,
                    ..Text::default()
                });
            }
        }

        if self.points.is_empty() {
            frame.fill_text(Text {
                content: "no keyframes yet, add one".to_string(),
                position: Point::new(plot.center_x(), plot.center_y()),
                color: MUTED,
                size: 13.0.into(),
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Center,
                ..Text::default()
            });
            return vec![frame.into_geometry()];
        }

        let samples = curve::samples(self.points, 10);
        let accent = if self.enabled { ACCENT } else { MUTED };

        // Area under the curve.
        let mut area = canvas::path::Builder::new();
        area.move_to(Point::new(
            self.x_of(plot, 0),
            self.y_of(plot, 0),
        ));
        for (minute, value) in &samples {
            area.line_to(Point::new(
                self.x_of(plot, *minute),
                self.y_of(plot, *value),
            ));
        }
        area.line_to(Point::new(
            self.x_of(plot, 1440),
            self.y_of(plot, 0),
        ));
        area.close();
        frame.fill(&area.build(), Color { a: 0.13, ..accent });

        // Curve.
        let mut line = canvas::path::Builder::new();
        for (index, (minute, value)) in samples.iter().enumerate() {
            let point = Point::new(self.x_of(plot, *minute), self.y_of(plot, *value));
            if index == 0 {
                line.move_to(point);
            } else {
                line.line_to(point);
            }
        }
        frame.stroke(
            &line.build(),
            Stroke::default().with_color(accent).with_width(2.0),
        );

        // "now" marker.
        let now_x = self.x_of(plot, self.now);
        frame.stroke(
            &Path::line(Point::new(now_x, plot.y), Point::new(now_x, plot.y + plot.height)),
            Stroke::default()
                .with_color(Color { a: 0.45, ..GOLD })
                .with_width(1.0),
        );
        if let Some(value) = curve::value_at(self.points, self.now) {
            frame.fill(
                &Path::circle(Point::new(now_x, self.y_of(plot, value)), 3.5),
                GOLD,
            );
        }

        // Handles.
        for point in self.points {
            let center = Point::new(self.x_of(plot, point.minute), self.y_of(plot, point.value));
            let hovered = cursor
                .position_in(bounds)
                .map(|position| {
                    (position.x - center.x).abs() < 10.0 && (position.y - center.y).abs() < 10.0
                })
                .unwrap_or(false);
            let active = state.dragging == Some(point.id) || hovered;
            frame.fill(
                &Path::circle(center, if active { HANDLE_RADIUS + 1.5 } else { HANDLE_RADIUS }),
                if active { accent } else { BG },
            );
            frame.stroke(
                &Path::circle(center, if active { HANDLE_RADIUS + 1.5 } else { HANDLE_RADIUS }),
                Stroke::default().with_color(accent).with_width(2.0),
            );
        }

        // Hover readout.
        if let Some(position) = cursor.position_in(bounds) {
            if plot.contains(position) {
                let (minute, _) = self.map(position, bounds);
                if let Some(value) = curve::value_at(self.points, minute) {
                    frame.fill_text(Text {
                        content: format!("{}   {}%", curve::fmt(minute), value),
                        position: Point::new(plot.x + plot.width - 4.0, plot.y + 4.0),
                        color: TEXT,
                        size: 11.0.into(),
                        align_x: iced::alignment::Horizontal::Right.into(),
                        align_y: iced::alignment::Vertical::Top,
                        ..Text::default()
                    });
                }
            }
        }

        // Accent edge on the panel's left side, so the chart reads as a card.
        frame.fill_rectangle(
            Point::new(0.0, plot.y),
            Size::new(2.0, plot.height),
            ACCENT_SOFT,
        );

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.dragging.is_some() {
            return mouse::Interaction::Grabbing;
        }
        if let Some(position) = cursor.position_in(bounds) {
            if self.enabled && self.hit(position, bounds).is_some() {
                return mouse::Interaction::Grab;
            }
            if self.plot(bounds).contains(position) {
                return mouse::Interaction::Crosshair;
            }
        }
        mouse::Interaction::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Point;

    #[test]
    fn maps_pixels_to_curve() {
        let points = vec![Point { id: 1, minute: 600, value: 80 }];
        let chart = Chart { points: &points, now: 600, enabled: true };
        let bounds = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 1000.0,
            height: 400.0,
        };
        let plot = chart.plot(bounds);

        // Centre of the plot is midday-ish and 50 %.
        let (minute, value) = chart.map(plot.center(), bounds);
        assert!((minute as i32 - 720).abs() <= 15);
        assert!((value as i32 - 50).abs() <= 1);

        // The 10:00 handle is exactly where it is drawn.
        let center = iced::Point::new(
            chart.x_of(plot, 600),
            chart.y_of(plot, 80),
        );
        let hit = chart.hit(center, bounds).expect("handle hit");
        assert_eq!(hit.id, 1);
    }
}
