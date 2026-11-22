// SPDX-License-Identifier: GPL-3.0-only

mod app;
mod provider;
pub use provider::{DataFuture, SystemProvider, SystemSummary, Usage};
mod localize;
mod mcp;
mod service;

use localize::localize;

pub fn run_installed() -> cosmic::iced::Result {
    run(service::provider())
}

pub fn run_mcp() -> Result<(), Box<dyn std::error::Error>> {
    mcp::run()
}

pub fn run(provider: SystemProvider) -> cosmic::iced::Result {
    localize();
    let settings = cosmic::applet::Context::default()
        .window_settings()
        .no_main_window(true);
    cosmic::app::run::<app::WidgetRail>(settings, provider)
}
