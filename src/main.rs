mod app;
mod config;
mod matrix;
mod message;
mod state;
mod ui;

use cosmic::app::Settings;
use cosmic::iced::Size;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("cosmic_matrix=info".parse().unwrap()),
        )
        .init();

    let settings = Settings::default()
        .size(Size::new(1100., 700.))
        .size_limits(
            cosmic::iced::Limits::NONE
                .min_width(400.0)
                .min_height(300.0),
        );

    cosmic::app::run::<app::App>(settings, ())?;
    Ok(())
}
