// SPDX-License-Identifier: GPL-3.0-only

//! Finder-style window chrome: sidebar, toolbar and the "⋯" menu.

use cosmic::{
    Apply, Element,
    iced::{Alignment, Length},
    theme, widget,
};

use crate::glass;
use crate::nav::NavPage;
use crate::{App, ContextPage, Message, fl};

impl App {
    pub(crate) fn finder_sidebar(&self) -> Element<'_, Message> {
        let active = self
            .nav_model
            .active_data::<NavPage>()
            .copied()
            .unwrap_or_default();
        let updates_count = self.updates.as_ref().map_or(0, Vec::len)
            + self
                .yuos_catalog
                .as_ref()
                .and_then(|result| result.as_ref().ok())
                .map_or(0, |catalog| catalog.apps.iter().filter(|app| app.update).count());
        let item = |page: NavPage| -> Element<'_, Message> {
            let on = page == active;
            let mut row = widget::row::with_capacity(4)
                .push(page.icon().size(16))
                .push(widget::text::body(page.title()).width(Length::Fill))
                .spacing(8)
                .align_y(Alignment::Center);
            if page == NavPage::Updates && updates_count > 0 {
                row = row.push(
                    widget::container(widget::text::body(updates_count.to_string()).size(11))
                        .padding([0, 6])
                        .class(glass::badge_class(on)),
                );
            }
            widget::button::custom(row)
                .width(Length::Fill)
                .padding([5, 8])
                .class(glass::sidebar_item(on))
                .on_press(Message::NavSelect(page))
                .into()
        };
        let label = |text: String| -> Element<'_, Message> {
            widget::text::body(text)
                .size(11)
                .font(cosmic::font::semibold())
                .class(glass::tertiary_text())
                .apply(widget::container)
                .padding([12, 8, 4, 8])
                .into()
        };

        let mut search = widget::text_input::search_input(fl!("search"), &self.search_input)
            .id(self.search_id.clone())
            .on_input(Message::SearchInput)
            .on_submit(Message::SearchSubmit);
        if !self.search_input.is_empty() {
            search = search.on_clear(Message::SearchClear);
        }
        let mut column = widget::column::with_capacity(20)
            .push(search.apply(widget::container).padding([0, 0, 8, 0]))
            .push(item(NavPage::YuOsApps))
            .push(item(NavPage::Explore))
            .push(label(fl!("categories")));
        for page in [
            NavPage::Create,
            NavPage::Work,
            NavPage::Develop,
            NavPage::Learn,
            NavPage::Game,
            NavPage::Relax,
            NavPage::Socialize,
            NavPage::Utilities,
            NavPage::Applets,
        ] {
            column = column.push(item(page));
        }
        column = column
            .push(label(fl!("library")))
            .push(item(NavPage::Installed))
            .push(item(NavPage::Updates))
            .spacing(1);

        let mut sidebar = widget::column::with_capacity(2).push(
            widget::scrollable(column.padding([12, 10, 12, 10])).height(Length::Fill),
        );
        if let Some(summary) = self.operations_summary() {
            sidebar = sidebar
                .push(glass::hairline())
                .push(widget::container(summary).padding([10, 14, 12, 14]));
        }
        widget::container(sidebar)
            .width(Length::Fixed(232.0))
            .height(Length::Fill)
            .class(theme::Container::custom(glass::sidebar))
            .into()
    }

    pub(crate) fn operations_summary(&self) -> Option<Element<'_, Message>> {
        if self.progress_operations.is_empty() || self.pending_operations.is_empty() {
            return None;
        }
        let mut title = String::new();
        let mut total_progress = 0.0;
        let count = self.pending_operations.len();
        for (op, progress) in self.pending_operations.values() {
            if title.is_empty() {
                title = op.pending_text((*progress * 100.0) as i32);
            }
            total_progress += progress;
        }
        total_progress /= count as f32;
        if count > 1 {
            title = fl!(
                "operations-running",
                running = count,
                percent = (total_progress as i32)
            );
        }
        let bar = widget::determinate_linear(total_progress)
            .width(Length::Fill)
            .girth(Length::Fixed(4.0));
        Some(
            widget::column::with_children(vec![
                bar.into(),
                widget::text::body(title).size(12).into(),
                widget::row::with_children(vec![
                    widget::button::custom(widget::text::body(fl!("details")).size(12))
                        .padding(0)
                        .class(glass::link())
                        .on_press(Message::ToggleContextPage(ContextPage::Operations))
                        .into(),
                    widget::space::horizontal().width(Length::Fill).into(),
                    widget::button::custom(widget::text::body(fl!("dismiss")).size(12))
                        .padding(0)
                        .class(glass::link())
                        .on_press(Message::PendingDismiss)
                        .into(),
                ])
                .align_y(Alignment::Center)
                .into(),
            ])
            .spacing(7)
            .into(),
        )
    }

    pub(crate) fn finder_toolbar(&self) -> Element<'_, Message> {
        let title = widget::mouse_area(
            widget::container(
                widget::text::body(self.page_title())
                    .size(15)
                    .font(cosmic::font::semibold())
                    .wrapping(cosmic::iced::core::text::Wrapping::None),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Alignment::Center)
            .clip(true),
        )
        .on_drag(Message::WindowDrag)
        .on_double_click(Message::WindowToggleMaximize);

        let tool = |name: &'static str, message: Message| -> Element<'_, Message> {
            widget::button::custom(widget::icon::from_name(name).size(16))
                .padding([5, 6])
                .class(glass::tool())
                .on_press(message)
                .into()
        };
        let window_button = |name: &'static str, message: Message, close: bool| {
            widget::button::custom(widget::icon::from_name(name).size(14))
                .padding([5, 6])
                .class(if close { glass::close_tool() } else { glass::tool() })
                .on_press(message)
        };

        let mut row = widget::row::with_capacity(10)
            .push(title)
            .spacing(6)
            .align_y(Alignment::Center);
        if self.fetching_backends
            || self.update_apps_scheduled
            || self.update_apps_in_progress
            || self.explore_results_handle.is_some()
            || !self.pending_backend_updates.is_empty()
            || !self.pending_operations.is_empty()
            || !self.waiting_installed.is_empty()
            || !self.waiting_updates.is_empty()
        {
            row = row.push(widget::indeterminate_circular().size(16.0));
        }
        row = row
            .push(
                widget::button::custom(
                    widget::row::with_children(vec![
                        widget::icon::from_name("chat-symbolic").size(16).into(),
                        widget::text::body(fl!("ask-yu-short")).into(),
                    ])
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .padding([5, 8])
                .class(glass::tool())
                .on_press(Message::AskYu),
            )
            .push(tool(
                "folder-remote-symbolic",
                Message::ToggleContextPage(ContextPage::Repositories),
            ))
            .push({
                let mut more = widget::popover(tool("view-more-symbolic", Message::MenuToggle))
                    .position(widget::popover::Position::Bottom)
                    .on_close(Message::MenuClose);
                if self.menu_open {
                    more = more.popup(self.more_menu());
                }
                more
            })
            .push(
                widget::container(
                    cosmic::iced::widget::rule::vertical(1).class(glass::rule()),
                )
                .height(Length::Fixed(18.0))
                .padding([0, 4]),
            )
            .push(window_button(
                "window-minimize-symbolic",
                Message::WindowMinimize,
                false,
            ))
            .push(window_button(
                "window-maximize-symbolic",
                Message::WindowToggleMaximize,
                false,
            ))
            .push(window_button(
                "window-close-symbolic",
                Message::WindowClose,
                true,
            ));
        widget::container(row)
            .height(Length::Fixed(52.0))
            .padding([0, 10, 0, 28])
            .into()
    }

    fn more_menu(&self) -> Element<'_, Message> {
        let entry = |icon: &'static str, label: String, message: Message| -> Element<'_, Message> {
            widget::button::custom(
                widget::row::with_children(vec![
                    widget::icon::from_name(icon).size(16).into(),
                    widget::text::body(label).into(),
                ])
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding([6, 10])
            .class(glass::menu_item())
            .on_press(Message::MenuAction(Box::new(message)))
            .into()
        };
        widget::container(
            widget::column::with_children(vec![
                entry("chat-symbolic", fl!("ask-yu-short"), Message::AskYu),
                entry(
                    "folder-remote-symbolic",
                    fl!("manage-repositories"),
                    Message::ToggleContextPage(ContextPage::Repositories),
                ),
                entry(
                    "emblem-synchronizing-symbolic",
                    fl!("operations"),
                    Message::ToggleContextPage(ContextPage::Operations),
                ),
                entry(
                    "preferences-system-symbolic",
                    fl!("settings"),
                    Message::ToggleContextPage(ContextPage::Settings),
                ),
                glass::hairline(),
                entry(
                    "system-search-symbolic",
                    fl!("search"),
                    Message::SearchActivate,
                ),
                entry("window-close-symbolic", fl!("close"), Message::WindowClose),
            ])
            .spacing(1),
        )
        .padding(5)
        .width(Length::Fixed(230.0))
        .class(theme::Container::custom(glass::menu))
        .into()
    }
}
