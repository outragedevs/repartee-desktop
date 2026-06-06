//! Repartee GUI — native iced desktop frontend (MVP).
//!
//! Reuses the `irc-repartee` protocol crate and a few of repartee's pure helpers
//! (nick colors, mIRC strip); the terminal/Unix-only parts of repartee
//! (ratatui, crossterm, fork/daemon, PTY shell, session sockets) are
//! deliberately not linked, so this binary is Windows-clean by construction.

mod app;
mod config;
mod format;
mod irc_client;
mod state;
mod theme;
mod ui;

use app::App;

pub fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "repartee_gui=info,wgpu=error,iced=warn".into()),
        )
        .init();

    iced::application(App::title, App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        .font(theme::FIRA_REGULAR)
        .font(theme::FIRA_BOLD)
        .default_font(theme::font())
        .window_size(iced::Size::new(1100.0, 720.0))
        .run_with(App::new)
}
