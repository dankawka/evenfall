mod app;

pub use app::Message;
mod chart;
mod config;
mod curve;
mod style;
mod system;
mod view;

use iced::window;

fn main() -> iced::Result {
    // Debug helper: print the config exactly as the UI would save it.
    if std::env::args().any(|arg| arg == "--print-config") {
        let cfg = config::Config::load().unwrap_or_default();
        print!("{}", cfg.render());
        return Ok(());
    }

    iced::application(app::boot, app::update, view::view)
        .title("evenfall — display brightness")
        .theme(view::theme)
        .subscription(app::subscription)
        .antialiasing(true)
        .window(window::Settings {
            icon: window_icon(),
            size: iced::Size::new(1180.0, 780.0),
            min_size: Some(iced::Size::new(940.0, 620.0)),
            ..Default::default()
        })
        .run()
}

fn window_icon() -> Option<window::Icon> {
    window::icon::from_rgba(include_bytes!("../assets/evenfall-64.rgba").to_vec(), 64, 64).ok()
}
