// SPDX-License-Identifier: GPL-3.0-only

use crate::{SystemProvider, SystemSummary, Usage, fl};
use cosmic::{
    Element, Task as IcedTask, app,
    cosmic_theme::palette::WithAlpha,
    iced::{
        Alignment, Background, Border, Color, Length, Limits, Shadow, Subscription, Vector, time,
        platform_specific::shell::wayland::commands::layer_surface::{
            Anchor, KeyboardInteractivity, Layer, get_layer_surface,
        },
        runtime::platform_specific::wayland::layer_surface::SctkLayerSurfaceSettings,
        widget::{column, container, row, scrollable, space::horizontal as horizontal_space, text},
        window,
    },
    theme,
    widget::icon,
};
use std::time::Duration;

const REFRESH_SECONDS: u64 = 5;
const RAIL_WIDTH: u32 = 360;
const RAIL_HEIGHT: f32 = 620.0;

#[derive(Clone)]
enum SystemState {
    Loading,
    Ready(SystemSummary),
    Unavailable(String),
}

#[derive(Clone)]
pub struct WidgetRail {
    core: cosmic::app::Core,
    system: SystemState,
    provider: SystemProvider,
    system_in_flight: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    SystemLoaded(Result<SystemSummary, String>),
}

impl cosmic::Application for WidgetRail {
    type Message = Message;
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = SystemProvider;

    const APP_ID: &'static str = "com.yuos.AppletWidgetRail";

    fn init(mut core: cosmic::app::Core, provider: SystemProvider) -> (Self, app::Task<Message>) {
        let panel = SctkLayerSurfaceSettings {
            id: window::Id::RESERVED,
            namespace: Self::APP_ID.into(),
            layer: Layer::Top,
            keyboard_interactivity: KeyboardInteractivity::None,
            anchor: Anchor::TOP | Anchor::RIGHT,
            size: Some((Some(RAIL_WIDTH), Some(RAIL_HEIGHT as u32 + 8))),
            ..Default::default()
        };
        core.set_main_window_id(Some(panel.id));
        core.window.show_headerbar = false;
        core.window.sharp_corners = true;
        core.window.show_maximize = false;
        core.window.show_minimize = false;
        core.window.use_template = false;
        let refresh = refresh_system(&provider);
        let rail = Self {
            core,
            provider,
            system: SystemState::Loading,
            system_in_flight: true,
        };
        (rail, IcedTask::batch([get_layer_surface(panel), refresh]))
    }

    fn core(&self) -> &cosmic::app::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::app::Core {
        &mut self.core
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }

    fn subscription(&self) -> Subscription<Message> {
        time::every(Duration::from_secs(REFRESH_SECONDS)).map(|_| Message::Refresh)
    }

    fn update(&mut self, message: Message) -> app::Task<Message> {
        match message {
            Message::Refresh => {
                if self.system_in_flight {
                    IcedTask::none()
                } else {
                    self.system_in_flight = true;
                    refresh_system(&self.provider)
                }
            }
            Message::SystemLoaded(result) => {
                self.system_in_flight = false;
                self.system = match result {
                    Ok(summary) => SystemState::Ready(summary),
                    Err(error) => SystemState::Unavailable(short_error(error)),
                };
                IcedTask::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let content = column![
            rail_header(),
            self.system_card(),
            self.suggestions_card(),
        ]
        .spacing(12)
        .width(Length::Fill);

        let content = container(
            scrollable(content)
                .height(Length::Fixed(RAIL_HEIGHT))
                .width(Length::Fill),
        )
        .width(Length::Fixed(RAIL_WIDTH as f32))
        .padding(4);

        let mut limits = Limits::NONE.min_width(1.0).min_height(1.0);
        if let Some(bounds) = self.core.applet.suggested_bounds {
            if bounds.width > 0.0 {
                limits = limits.max_width(bounds.width);
            }
            if bounds.height > 0.0 {
                limits = limits.max_height(bounds.height);
            }
        }
        self.core
            .applet
            .autosize_window(content)
            .limits(limits)
            .into()
    }
}

impl WidgetRail {
    fn system_card(&self) -> Element<'_, Message> {
        let body: Element<'_, Message> = match &self.system {
            SystemState::Loading => state_row("process-working-symbolic", fl!("system-loading")),
            SystemState::Unavailable(error) => {
                unavailable(fl!("system-unavailable"), error.to_string())
            }
            SystemState::Ready(summary) => {
                let cpu = summary
                    .cpu_percent
                    .map(|value| format!("{value:.0}%"))
                    .unwrap_or_else(|| fl!("warming-up"));
                let memory = summary
                    .memory
                    .map(format_usage)
                    .unwrap_or_else(|| fl!("not-available"));
                let storage = summary
                    .storage
                    .map(format_usage)
                    .unwrap_or_else(|| fl!("not-available"));
                let network = match (summary.network_down_bps, summary.network_up_bps) {
                    (Some(down), Some(up)) => fl!(
                        "network-rate",
                        down = format_rate(down),
                        up = format_rate(up)
                    ),
                    _ => fl!("warming-up"),
                };

                column![
                    row![metric(fl!("cpu"), cpu), metric(fl!("memory"), memory),].spacing(8),
                    row![
                        metric(fl!("storage"), storage),
                        metric(fl!("network"), network),
                    ]
                    .spacing(8),
                ]
                .spacing(8)
                .into()
            }
        };
        card(
            fl!("system-title"),
            "utilities-system-monitor-symbolic",
            body,
        )
    }

    fn suggestions_card(&self) -> Element<'_, Message> {
        let suggestions = match &self.system {
            SystemState::Ready(summary) => suggestions(summary),
            _ => Vec::new(),
        };
        let body: Element<'_, Message> = if matches!(self.system, SystemState::Loading) {
            state_row("process-working-symbolic", fl!("suggestions-loading"))
        } else if suggestions.is_empty() {
            state_row("dialog-information-symbolic", fl!("not-available"))
        } else {
            let mut content = column![].spacing(8);
            for suggestion in suggestions {
                content = content.push(
                    row![
                        status_dot(),
                        text(suggestion).size(12).width(Length::Fill),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Start),
                );
            }
            content.into()
        };
        card(fl!("suggestions-title"), "starred-symbolic", body)
    }
}

fn rail_header() -> Element<'static, Message> {
    container(
        column![
            text(fl!("rail-eyebrow")).size(10),
            text(fl!("rail-title")).size(24),
        ]
        .spacing(1)
        .padding([2, 4, 4, 4]),
    )
    .width(Length::Fill)
    .class(theme::Container::custom(glass_card_style))
    .into()
}

fn card<'a>(
    title: String,
    icon_name: &'static str,
    body: Element<'a, Message>,
) -> Element<'a, Message> {
    let header = row![
        icon::from_name(icon_name).size(17).symbolic(true),
        text(title).size(14),
        horizontal_space(),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    container(column![header, body].spacing(10))
        .width(Length::Fill)
        .padding(14)
        .class(theme::Container::custom(glass_card_style))
        .into()
}

fn glass_card_style(theme: &cosmic::Theme) -> container::Style {
    let cosmic = theme.cosmic();
    let mut fill = cosmic.bg_component_color();
    fill.alpha = if theme.theme_type.is_dark() {
        0.66
    } else {
        0.72
    };

    container::Style {
        text_color: Some(cosmic.on_bg_color().into()),
        icon_color: Some(cosmic.on_bg_color().into()),
        background: Some(Background::Color(fill.into())),
        border: Border {
            radius: cosmic.radius_l().into(),
            width: 1.0,
            color: cosmic.on_bg_color().with_alpha(0.10).into(),
        },
        shadow: Shadow {
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: if theme.theme_type.is_dark() {
                    0.30
                } else {
                    0.14
                },
            },
            offset: Vector { x: 0.0, y: 6.0 },
            blur_radius: 24.0,
        },
        snap: true,
    }
}

fn metric(label: String, value: String) -> Element<'static, Message> {
    container(column![text(label).size(10), text(value).size(12)].spacing(3))
        .width(Length::FillPortion(1))
        .padding([7, 8])
        .class(theme::Container::custom(|theme| {
            let cosmic = theme.cosmic();
            let mut fill = cosmic.bg_component_color();
            fill.alpha = 0.18;
            container::Style {
                background: Some(Background::Color(fill.into())),
                border: Border {
                    radius: cosmic.radius_s().into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }))
        .into()
}

fn state_row(icon_name: &'static str, message: String) -> Element<'static, Message> {
    row![
        icon::from_name(icon_name).size(16).symbolic(true),
        text(message).size(12).width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn unavailable(title: String, detail: String) -> Element<'static, Message> {
    row![
        icon::from_name("dialog-warning-symbolic")
            .size(16)
            .symbolic(true),
        column![text(title).size(12), text(detail).size(10)].spacing(2),
    ]
    .spacing(8)
    .align_y(Alignment::Start)
    .into()
}

fn status_dot() -> Element<'static, Message> {
    const SIZE: f32 = 7.0;
    container(horizontal_space().width(Length::Fixed(SIZE)))
        .height(Length::Fixed(SIZE))
        .class(theme::Container::custom(|theme| {
            container::Style {
                background: Some(Background::Color(theme.cosmic().accent_color().into())),
                border: Border {
                    radius: (SIZE / 2.0).into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }))
        .into()
}

fn suggestions(summary: &SystemSummary) -> Vec<String> {
    let memory = summary.memory.filter(|usage| usage.total_mb > 0);
    let storage = summary.storage.filter(|usage| usage.total_mb > 0);
    let mut suggestions = Vec::new();
    if memory.is_some_and(|usage| usage.percent() >= 85) {
        suggestions.push(fl!("suggestion-memory"));
    }
    if storage.is_some_and(|usage| usage.percent() >= 90) {
        suggestions.push(fl!("suggestion-storage"));
    }
    if suggestions.is_empty() && memory.is_some() && storage.is_some() {
        suggestions.push(fl!("suggestion-balanced"));
    }
    suggestions
}

fn format_usage(usage: Usage) -> String {
    format!(
        "{}% · {:.1} GB",
        usage.percent(),
        usage.used_mb as f64 / 1024.0
    )
}

fn format_rate(bytes_per_second: u64) -> String {
    if bytes_per_second >= 1_048_576 {
        format!("{:.1} MB/s", bytes_per_second as f64 / 1_048_576.0)
    } else if bytes_per_second >= 1024 {
        format!("{:.0} KB/s", bytes_per_second as f64 / 1024.0)
    } else {
        format!("{bytes_per_second} B/s")
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let prefix: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

fn short_error(error: String) -> String {
    truncate(error.lines().next().unwrap_or_default().trim(), 96)
}

fn refresh_system(provider: &SystemProvider) -> app::Task<Message> {
    IcedTask::perform(provider(), |result| {
        cosmic::Action::App(Message::SystemLoaded(result))
    })
}

#[cfg(test)]
mod tests {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test/unit/app.rs"
    ));
}
