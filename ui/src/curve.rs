//! Brightness interpolation — must match the Python daemon exactly.

use crate::config::Point;

pub fn value_at(points: &[Point], minute: u16) -> Option<u8> {
    if points.is_empty() {
        return None;
    }
    if points.len() == 1 {
        return Some(points[0].value);
    }
    let last = points[points.len() - 1];
    let head = points[0];

    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if a.minute <= minute && minute <= b.minute {
            return Some(lerp(a.minute, a.value, b.minute, b.value, minute));
        }
    }

    // Across midnight, from the last keyframe back to the first one.
    let span = (1440 - last.minute) as f32 + head.minute as f32;
    if span <= 0.0 {
        return Some(last.value);
    }
    let pos = ((minute as i32 - last.minute as i32).rem_euclid(1440)) as f32;
    Some(clamp(
        last.value as f32 + (head.value as f32 - last.value as f32) * pos / span,
    ))
}

pub fn next_change(points: &[Point], minute: u16) -> Option<(u16, u8)> {
    points
        .iter()
        .find(|p| p.minute > minute)
        .or(points.first())
        .map(|p| (p.minute, p.value))
}

pub fn samples(points: &[Point], step: u16) -> Vec<(u16, u8)> {
    if points.is_empty() {
        return Vec::new();
    }
    (0..=1440 / step)
        .filter_map(|i| {
            let minute = i * step;
            value_at(points, minute.min(1439)).map(|value| (minute, value))
        })
        .collect()
}

pub fn lerp(m0: u16, v0: u8, m1: u16, v1: u8, minute: u16) -> u8 {
    if m0 == m1 {
        return v0;
    }
    let t = (minute as f32 - m0 as f32) / (m1 as f32 - m0 as f32);
    clamp(v0 as f32 + (v1 as f32 - v0 as f32) * t)
}

fn clamp(value: f32) -> u8 {
    // Banker's rounding, to stay byte-identical with the Python daemon.
    value.round_ties_even().clamp(0.0, 100.0) as u8
}

pub fn fmt(minute: u16) -> String {
    format!("{:02}:{:02}", minute / 60, minute % 60)
}

pub fn parse_time(text: &str) -> Option<u16> {
    let text = text.trim();
    let (hours, minutes) = text.split_once(':')?;
    let hours: u16 = hours.trim().parse().ok()?;
    let minutes: u16 = minutes.trim().parse().ok()?;
    if minutes > 59 || hours > 24 || (hours == 24 && minutes > 0) {
        return None;
    }
    Some(((hours * 60 + minutes) % 1440) as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Point;

    fn points() -> Vec<Point> {
        vec![
            Point { id: 1, minute: 360, value: 30 },
            Point { id: 2, minute: 600, value: 80 },
            Point { id: 3, minute: 960, value: 30 },
        ]
    }

    #[test]
    fn matches_daemon_curve() {
        let points = points();
        assert_eq!(value_at(&points, 360), Some(30));
        assert_eq!(value_at(&points, 420), Some(42));
        assert_eq!(value_at(&points, 480), Some(55));
        assert_eq!(value_at(&points, 540), Some(68));
        assert_eq!(value_at(&points, 600), Some(80));
        assert_eq!(value_at(&points, 720), Some(63));
        assert_eq!(value_at(&points, 960), Some(30));
        // 16:00 -> 06:00 must stay flat at the base value.
        assert_eq!(value_at(&points, 0), Some(30));
        assert_eq!(value_at(&points, 1439), Some(30));
    }

    #[test]
    fn next_change_wraps() {
        let points = points();
        assert_eq!(next_change(&points, 600), Some((960, 30)));
        assert_eq!(next_change(&points, 1200), Some((360, 30)));
    }
}
