//! Palette and widget styles, sharing the look of the other desktop tools.

use iced::border::Radius;
use iced::widget::{button, container, text_input};
use iced::{Border, Color, Font, Theme};

pub const BG: Color = Color::from_rgb8(0x12, 0x14, 0x1a);
pub const SURFACE: Color = Color::from_rgb8(0x1a, 0x1d, 0x26);
pub const SURFACE2: Color = Color::from_rgb8(0x22, 0x26, 0x32);
pub const BORDER: Color = Color::from_rgb8(0x2e, 0x34, 0x44);
pub const TEXT: Color = Color::from_rgb8(0xe8, 0xea, 0xef);
pub const MUTED: Color = Color::from_rgb8(0x8b, 0x93, 0xa7);
pub const ACCENT: Color = Color::from_rgb8(0x5b, 0x9f, 0xd4);
pub const ACCENT_SOFT: Color = Color::from_rgb8(0x20, 0x33, 0x47);
pub const GOLD: Color = Color::from_rgb8(0xe0, 0xb3, 0x4a);
pub const OK: Color = Color::from_rgb8(0x3d, 0x9b, 0x6a);
pub const WARN: Color = Color::from_rgb8(0xc9, 0xa2, 0x27);
pub const WARN_BG: Color = Color::from_rgb8(0x3d, 0x34, 0x20);
pub const OK_BG: Color = Color::from_rgb8(0x1a, 0x3d, 0x2a);
pub const BAD_BG: Color = Color::from_rgb8(0x3d, 0x22, 0x22);
pub const BAD: Color = Color::from_rgb8(0xc4, 0x5c, 0x5c);

pub fn theme() -> Theme {
    Theme::custom(
        "evenfall".to_string(),
        iced::theme::Palette {
            background: BG,
            text: TEXT,
            primary: ACCENT,
            success: OK,
            danger: BAD,
            warning: WARN,
        },
    )
}

pub fn bold() -> Font {
    Font {
        weight: iced::font::Weight::Bold,
        ..Font::DEFAULT
    }
}

fn radius(r: f32) -> Radius {
    r.into()
}

pub fn panel(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE.into()),
        text_color: Some(TEXT),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: radius(12.0),
        },
        ..container::Style::default()
    }
}

pub fn inset(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(BG.into()),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: radius(8.0),
        },
        ..container::Style::default()
    }
}

pub fn header(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE.into()),
        text_color: Some(TEXT),
        ..container::Style::default()
    }
}

pub fn chip(background: Color, text: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(background.into()),
        text_color: Some(text),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius(7.0),
        },
        ..container::Style::default()
    }
}

pub fn monitor_row(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (background, border) = if selected {
            (ACCENT_SOFT, ACCENT)
        } else {
            match status {
                button::Status::Hovered => (SURFACE2, BORDER),
                button::Status::Pressed => (ACCENT_SOFT, ACCENT),
                _ => (Color::TRANSPARENT, Color::TRANSPARENT),
            }
        };
        button::Style {
            background: Some(background.into()),
            text_color: TEXT,
            border: Border {
                color: border,
                width: if border == Color::TRANSPARENT { 0.0 } else { 1.0 },
                radius: radius(8.0),
            },
            ..button::Style::default()
        }
    }
}

pub fn ghost(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => SURFACE2,
        button::Status::Pressed => ACCENT_SOFT,
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(background.into()),
        text_color: TEXT,
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: radius(8.0),
        },
        ..button::Style::default()
    }
}

pub fn segment(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (background, border, text) = if selected {
            (ACCENT_SOFT, ACCENT, TEXT)
        } else {
            match status {
                button::Status::Hovered => (SURFACE2, BORDER, TEXT),
                button::Status::Pressed => (ACCENT_SOFT, ACCENT, TEXT),
                _ => (Color::TRANSPARENT, BORDER, MUTED),
            }
        };
        button::Style {
            background: Some(background.into()),
            text_color: text,
            border: Border {
                color: border,
                width: 1.0,
                radius: radius(7.0),
            },
            ..button::Style::default()
        }
    }
}

pub fn quiet(_theme: &Theme, status: button::Status) -> button::Style {
    let color = match status {
        button::Status::Hovered => TEXT,
        button::Status::Pressed => ACCENT,
        _ => MUTED,
    };
    button::Style {
        background: Some(Color::TRANSPARENT.into()),
        text_color: color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius(6.0),
        },
        ..button::Style::default()
    }
}

pub fn danger(_theme: &Theme, status: button::Status) -> button::Style {
    let color = match status {
        button::Status::Hovered => BAD,
        button::Status::Pressed => TEXT,
        _ => MUTED,
    };
    button::Style {
        background: Some(Color::TRANSPARENT.into()),
        text_color: color,
        border: Border::default(),
        ..button::Style::default()
    }
}

pub fn input(_theme: &Theme, _status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: BG.into(),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: radius(7.0),
        },
        icon: MUTED,
        placeholder: MUTED,
        value: TEXT,
        selection: ACCENT,
    }
}
