// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    HistoryPermission, HistoryPolicy,
    copyq::{self, ClipboardEntry},
    fl,
};
use cosmic::{
    Element, Task, app,
    cosmic_theme::palette::WithAlpha,
    iced::{
        Alignment, Background, Border, Length,
        core::text::{Ellipsize, EllipsizeHeightLimit},
        platform_specific::shell::wayland::commands::{
            layer_surface::{Anchor, KeyboardInteractivity, Layer, get_layer_surface},
            popup::{destroy_popup, get_popup},
        },
        runtime::platform_specific::wayland::layer_surface::SctkLayerSurfaceSettings,
        widget::{column, container, opaque, row, scrollable, stack},
        window,
    },
    surface, theme,
    widget::{button, divider, icon, space, text},
};
use jiff::{Timestamp, civil::DateTime, tz::TimeZone};

#[derive(Clone)]
enum HistoryState {
    Loading,
    Ready(Vec<ClipboardEntry>),
    Unavailable(String),
}

pub struct ClipboardApplet {
    core: cosmic::app::Core,
    popup: Option<window::Id>,
    history: HistoryState,
    selected: Option<usize>,
    confirm_clear: bool,
    notice: Option<String>,
    refresh_generation: u64,
    busy: bool,
    policy: Option<HistoryPolicy>,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    CloseRequested(window::Id),
    Refresh,
    Loaded(u64, Result<Vec<ClipboardEntry>, String>),
    Select(usize),
    Restore(String),
    Delete(String),
    AskClear,
    CancelClear,
    ConfirmClear,
    ActionFinished(u64, Action, Result<(), String>),
    Surface(surface::Action),
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Restore,
    Delete,
    Clear,
}

impl cosmic::Application for ClipboardApplet {
    type Message = Message;
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = Option<HistoryPolicy>;

    const APP_ID: &'static str = "com.yuos.AppletClipboard";

    fn init(
        mut core: cosmic::app::Core,
        policy: Option<HistoryPolicy>,
    ) -> (Self, app::Task<Message>) {
        let (width, height) = core.applet.suggested_window_size();
        let panel = SctkLayerSurfaceSettings {
            id: window::Id::RESERVED,
            namespace: Self::APP_ID.into(),
            layer: Layer::Top,
            keyboard_interactivity: KeyboardInteractivity::None,
            anchor: Anchor::TOP | Anchor::RIGHT,
            size: Some((Some(width.get()), Some(height.get()))),
            ..Default::default()
        };
        core.set_main_window_id(Some(panel.id));
        core.window.show_headerbar = false;
        core.window.sharp_corners = true;
        core.window.show_maximize = false;
        core.window.show_minimize = false;
        core.window.use_template = false;
        (
            Self {
                core,
                popup: None,
                history: HistoryState::Ready(Vec::new()),
                selected: None,
                confirm_clear: false,
                notice: None,
                refresh_generation: 0,
                busy: false,
                policy,
            },
            get_layer_surface(panel),
        )
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

    fn update(&mut self, message: Message) -> app::Task<Message> {
        match message {
            Message::TogglePopup => {
                if let Some(id) = self.popup.take() {
                    self.clear_popup_state();
                    destroy_popup(id)
                } else {
                    if self.busy {
                        return Task::none();
                    }
                    self.history = HistoryState::Loading;
                    self.notice = None;
                    self.busy = true;
                    self.refresh_generation = self.refresh_generation.wrapping_add(1);
                    let id = window::Id::unique();
                    self.popup = Some(id);
                    let settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().expect("panel window"),
                        id,
                        Some((380, 600)),
                        None,
                        None,
                    );
                    Task::batch([
                        get_popup(settings),
                        refresh(self.refresh_generation, self.policy),
                    ])
                }
            }
            Message::CloseRequested(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                    self.clear_popup_state();
                }
                Task::none()
            }
            Message::Refresh => {
                if self.busy || self.popup.is_none() {
                    return Task::none();
                }
                self.history = HistoryState::Loading;
                self.selected = None;
                self.confirm_clear = false;
                self.notice = None;
                self.busy = true;
                self.refresh_generation = self.refresh_generation.wrapping_add(1);
                refresh(self.refresh_generation, self.policy)
            }
            Message::Loaded(generation, result) => {
                if generation != self.refresh_generation {
                    return Task::none();
                }
                self.busy = false;
                if self.popup.is_none() {
                    self.refresh_generation = self.refresh_generation.wrapping_add(1);
                    return Task::none();
                }
                self.history = match result {
                    Ok(entries) => HistoryState::Ready(entries),
                    Err(error) => HistoryState::Unavailable(short_error(&error)),
                };
                Task::none()
            }
            Message::Select(index) => {
                if !self.busy
                    && !self.confirm_clear
                    && matches!(&self.history, HistoryState::Ready(entries) if index < entries.len())
                {
                    self.selected = Some(index);
                }
                Task::none()
            }
            Message::Restore(identity) => {
                if self.busy || self.popup.is_none() {
                    return Task::none();
                }
                self.busy = true;
                self.notice = None;
                self.refresh_generation = self.refresh_generation.wrapping_add(1);
                action(
                    self.refresh_generation,
                    Action::Restore,
                    restore(self.policy, identity),
                )
            }
            Message::Delete(identity) => {
                if self.busy || self.popup.is_none() {
                    return Task::none();
                }
                self.busy = true;
                self.notice = None;
                self.refresh_generation = self.refresh_generation.wrapping_add(1);
                action(
                    self.refresh_generation,
                    Action::Delete,
                    remove(self.policy, identity),
                )
            }
            Message::AskClear => {
                if !self.busy {
                    self.confirm_clear = true;
                }
                Task::none()
            }
            Message::CancelClear => {
                if !self.busy {
                    self.confirm_clear = false;
                }
                Task::none()
            }
            Message::ConfirmClear => {
                if self.busy || self.popup.is_none() {
                    return Task::none();
                }
                self.confirm_clear = false;
                self.busy = true;
                self.notice = None;
                self.refresh_generation = self.refresh_generation.wrapping_add(1);
                action(self.refresh_generation, Action::Clear, clear(self.policy))
            }
            Message::ActionFinished(generation, kind, result) => {
                if generation != self.refresh_generation {
                    return Task::none();
                }
                if self.popup.is_none() {
                    self.busy = false;
                    self.refresh_generation = self.refresh_generation.wrapping_add(1);
                    return Task::none();
                }
                match result {
                    Ok(()) => {
                        if matches!(kind, Action::Restore) {
                            self.busy = false;
                            self.notice = Some(fl!("restored"));
                            Task::none()
                        } else {
                            self.history = HistoryState::Loading;
                            self.selected = None;
                            self.refresh_generation = self.refresh_generation.wrapping_add(1);
                            refresh(self.refresh_generation, self.policy)
                        }
                    }
                    Err(error) => {
                        self.busy = false;
                        self.notice =
                            Some(format!("{}: {}", fl!("action-failed"), short_error(&error)));
                        Task::none()
                    }
                }
            }
            Message::Surface(action) => {
                cosmic::task::message(cosmic::Action::Cosmic(cosmic::app::Action::Surface(action)))
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let button = self
            .core
            .applet
            .icon_button("edit-paste-symbolic")
            .on_press_down(Message::TogglePopup);
        self.core
            .applet
            .applet_tooltip(
                button,
                format!("{} · CopyQ", fl!("tooltip")),
                self.popup.is_some(),
                Message::Surface,
                None,
            )
            .into()
    }

    fn view_window(&self, _id: window::Id) -> Element<'_, Message> {
        let enabled = !self.busy && !self.confirm_clear;
        let header = container(
            row![
                icon::from_name("edit-paste-symbolic")
                    .size(16)
                    .symbolic(true),
                text(format!("{} · CopyQ", fl!("title")))
                    .font(cosmic::font::semibold())
                    .size(15)
                    .width(Length::Fill),
                action_button("view-refresh-symbolic", fl!("refresh"))
                    .class(theme::Button::Icon)
                    .on_press_maybe(enabled.then_some(Message::Refresh)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([8, 12])
        .width(Length::Fill);

        let body: Element<'_, Message> = match &self.history {
            HistoryState::Loading => state_row("process-working-symbolic", fl!("loading"), None),
            HistoryState::Ready(entries) if entries.is_empty() => {
                state_row("edit-paste-symbolic", fl!("empty"), None)
            }
            HistoryState::Ready(entries) => {
                let mut list = column![].spacing(2);
                for (index, entry) in entries.iter().enumerate() {
                    list = list.push(history_row(
                        index,
                        entry,
                        self.selected == Some(index),
                        enabled,
                    ));
                }
                let detail = match self.selected.and_then(|index| entries.get(index)) {
                    Some(entry) => selected_detail(entry, enabled),
                    None => container(text(fl!("select-entry")).size(12))
                        .padding([10, 12])
                        .into(),
                };
                column![
                    container(
                        scrollable(list)
                            .height(Length::Fixed(290.0))
                            .width(Length::Fill)
                    )
                    .padding([4, 6]),
                    divider::horizontal::light(),
                    detail,
                    divider::horizontal::light(),
                    container(
                        destructive_action(fl!("clear-all"))
                            .on_press_maybe(enabled.then_some(Message::AskClear)),
                    )
                    .padding([4, 6]),
                ]
                .width(Length::Fill)
                .into()
            }
            HistoryState::Unavailable(error) => state_row(
                "dialog-warning-symbolic",
                fl!("unavailable"),
                Some(error.as_str()),
            ),
        };

        let mut content = column![header, divider::horizontal::light(), body,].width(Length::Fill);
        if let Some(notice) = &self.notice {
            content = content.push(
                container(text(notice).size(12))
                    .padding([6, 12])
                    .width(Length::Fill),
            );
        }
        content = content.push(divider::horizontal::light()).push(
            container(muted_text(fl!("privacy-note"), 11))
                .padding([8, 12])
                .width(Length::Fill),
        );

        let panel: Element<'_, Message> = if self.confirm_clear {
            stack![content, confirmation(self.busy)].into()
        } else {
            content.into()
        };
        self.core
            .applet
            .popup_container(panel)
            .limits(
                cosmic::iced::core::layout::Limits::NONE
                    .min_width(380.0)
                    .max_width(380.0)
                    .max_height(600.0),
            )
            .into()
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::CloseRequested(id))
    }
}

impl ClipboardApplet {
    fn clear_popup_state(&mut self) {
        self.history = HistoryState::Ready(Vec::new());
        self.selected = None;
        self.confirm_clear = false;
        self.notice = None;
        if !self.busy {
            self.refresh_generation = self.refresh_generation.wrapping_add(1);
        }
    }
}

fn refresh(generation: u64, policy: Option<HistoryPolicy>) -> app::Task<Message> {
    Task::perform(load_history(policy), move |result| {
        cosmic::Action::App(Message::Loaded(generation, result))
    })
}

fn action(
    generation: u64,
    kind: Action,
    future: impl std::future::Future<Output = Result<(), String>> + Send + 'static,
) -> app::Task<Message> {
    Task::perform(future, move |result| {
        cosmic::Action::App(Message::ActionFinished(generation, kind, result))
    })
}

async fn load_history(policy: Option<HistoryPolicy>) -> Result<Vec<ClipboardEntry>, String> {
    if let Some(policy) = policy {
        policy(HistoryPermission::Read).await?;
    }
    copyq::load_history().await
}

async fn restore(policy: Option<HistoryPolicy>, identity: String) -> Result<(), String> {
    if let Some(policy) = policy {
        policy(HistoryPermission::Read).await?;
        policy(HistoryPermission::Write).await?;
    }
    copyq::restore(identity).await
}

async fn remove(policy: Option<HistoryPolicy>, identity: String) -> Result<(), String> {
    if let Some(policy) = policy {
        policy(HistoryPermission::Read).await?;
        policy(HistoryPermission::Write).await?;
    }
    copyq::remove(identity).await
}

async fn clear(policy: Option<HistoryPolicy>) -> Result<(), String> {
    if let Some(policy) = policy {
        policy(HistoryPermission::Write).await?;
    }
    copyq::clear().await
}

fn action_button(icon_name: &'static str, label: String) -> button::Button<'static, Message> {
    button::custom(
        row![
            icon::from_name(icon_name).size(14).symbolic(true),
            text(label).size(13),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([4, 7])
    .class(theme::Button::ListItem([6.0; 4]))
}

fn destructive_action(label: String) -> button::Button<'static, Message> {
    button::custom(
        row![
            icon::from_name("edit-delete-symbolic")
                .size(14)
                .symbolic(true),
            text(label).size(13).class(theme::Text::Custom(|theme| {
                cosmic::iced::widget::text::Style {
                    color: Some(theme.cosmic().destructive_text_color().into()),
                }
            })),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([4, 7])
    .class(theme::Button::ListItem([6.0; 4]))
}

fn muted_text<'a>(
    label: impl Into<std::borrow::Cow<'a, str>> + 'a,
    size: u16,
) -> cosmic::widget::Text<'a, cosmic::Theme, cosmic::Renderer> {
    text(label).size(size).class(theme::Text::Custom(|theme| {
        cosmic::iced::widget::text::Style {
            color: Some(theme.cosmic().on_bg_color().with_alpha(0.74).into()),
        }
    }))
}

fn history_row(
    index: usize,
    entry: &ClipboardEntry,
    selected: bool,
    enabled: bool,
) -> Element<'_, Message> {
    let age = entry
        .copied_at
        .as_deref()
        .map(format_age)
        .filter(|value| !value.is_empty());
    let preview = if entry.preview.trim().is_empty() {
        fl!("blank-entry")
    } else {
        entry.preview.replace(['\n', '\r'], " ")
    };
    let mut heading = row![
        text(preview)
            .size(13)
            .width(Length::Fill)
            .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1))),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if let Some(age) = age {
        heading = heading.push(
            text(age)
                .size(11)
                .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1))),
        );
    }
    let class = if selected {
        theme::Button::Suggested
    } else {
        theme::Button::ListItem([6.0; 4])
    };
    button::custom(heading)
        .class(class)
        .width(Length::Fill)
        .padding([5, 8])
        .on_press_maybe(enabled.then_some(Message::Select(index)))
        .into()
}

fn selected_detail(entry: &ClipboardEntry, enabled: bool) -> Element<'_, Message> {
    let label = if let Some(time) = entry.copied_at.as_deref() {
        format!("{} · {}", fl!("selected-preview"), format_age(time))
    } else {
        fl!("selected-preview")
    };
    container(
        column![
            muted_text(label, 11),
            container(
                scrollable(text(entry.preview.as_str()).size(13).width(Length::Fill))
                    .width(Length::Fill),
            )
            .max_height(56),
            row![
                action_button("edit-paste-symbolic", fl!("restore"))
                    .class(theme::Button::Suggested)
                    .on_press_maybe(enabled.then(|| Message::Restore(entry.identity.clone()))),
                destructive_action(fl!("delete"))
                    .on_press_maybe(enabled.then(|| Message::Delete(entry.identity.clone()))),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        ]
        .spacing(5),
    )
    .padding([9, 12])
    .width(Length::Fill)
    .into()
}

fn confirmation(busy: bool) -> Element<'static, Message> {
    let dialog = container(
        column![
            text(fl!("clear-confirm-title"))
                .size(15)
                .font(cosmic::font::semibold()),
            text(fl!("clear-confirm-body")).size(13),
            row![
                space::horizontal().width(Length::Fill),
                button::text(fl!("cancel")).on_press_maybe((!busy).then_some(Message::CancelClear)),
                button::destructive(fl!("confirm-clear"))
                    .on_press_maybe((!busy).then_some(Message::ConfirmClear)),
            ]
            .spacing(6),
        ]
        .spacing(10),
    )
    .padding(14)
    .width(Length::Fixed(318.0))
    .class(theme::Container::custom(|theme| {
        let cosmic = theme.cosmic();
        container::Style {
            background: Some(Background::Color(cosmic.background.base.into())),
            border: Border {
                radius: cosmic.radius_m().into(),
                width: 1.0,
                color: cosmic.on_bg_color().with_alpha(0.14).into(),
            },
            ..Default::default()
        }
    }));
    opaque(
        container(dialog)
            .center(Length::Fill)
            .class(theme::Container::custom(|theme| container::Style {
                background: Some(Background::Color(
                    theme.cosmic().background.base.with_alpha(0.65).into(),
                )),
                ..Default::default()
            })),
    )
    .into()
}

fn state_row<'a>(
    icon_name: &'static str,
    label: String,
    detail: Option<&'a str>,
) -> Element<'a, Message> {
    let mut message = column![text(label).size(13).font(cosmic::font::semibold())].spacing(4);
    if let Some(detail) = detail {
        message = message.push(muted_text(detail, 12));
    }
    container(
        row![
            icon::from_name(icon_name).size(16).symbolic(true),
            message.width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([20, 14])
    .width(Length::Fill)
    .into()
}

fn format_age(value: &str) -> String {
    let epoch_seconds = value
        .parse::<i64>()
        .ok()
        .map(|raw| {
            if raw > 10_000_000_000 {
                raw / 1000
            } else {
                raw
            }
        })
        .or_else(|| value.parse::<Timestamp>().ok().map(|time| time.as_second()))
        .or_else(|| {
            value
                .parse::<DateTime>()
                .ok()?
                .to_zoned(TimeZone::system())
                .ok()
                .map(|zoned| zoned.timestamp().as_second())
        });
    let Some(epoch_seconds) = epoch_seconds else {
        return truncate(value, 24);
    };
    let seconds = Timestamp::now()
        .as_second()
        .saturating_sub(epoch_seconds)
        .max(0) as u64;
    let minutes = seconds / 60;
    if minutes < 60 {
        fl!("minutes-ago", count = minutes)
    } else if minutes < 60 * 24 {
        let hours = minutes / 60_u64;
        fl!("hours-ago", count = hours)
    } else {
        let days = minutes / (60_u64 * 24);
        fl!("days-ago", count = days)
    }
}

fn short_error(error: &str) -> String {
    truncate(error.lines().next().unwrap_or_default().trim(), 96)
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

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/test/unit/app.rs"));
}
