// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

use std::cmp;
use std::sync::Arc;

use cosmic::{
    Apply, Element, cosmic_theme,
    iced::{Alignment, Length, Size},
    theme, widget,
};
use rayon::prelude::*;

use crate::app_id::AppId;
use crate::app_info::{AppInfo, AppProvide, AppUrl};
use crate::backend::{BackendName, Package};
use crate::config::AppTheme;
use crate::explore::ExplorePage;
use crate::fl;
use crate::localize::LANGUAGE_SORTER;
use crate::nav::NavPage;
use crate::operation::OperationKind;
use crate::glass::Pill;
use crate::search::{GridMetrics, SearchResult};
use crate::{
    App, AppEntry, ContextPage, DialogPage, MAX_RESULTS,
    Message, SelectedSource, SourceKind,
};

/// Flat Finder-style app row: icon, name, secondary summary, trailing
/// actions and a bottom hairline.
pub fn app_row<'a>(
    name: &'a str,
    summary: &'a str,
    icon_opt: Option<&'a widget::icon::Handle>,
    icon_size: u16,
    actions: Vec<Element<'a, Message>>,
    width: usize,
) -> Element<'a, Message> {
    let icon: Element<_> = match icon_opt {
        Some(icon) => widget::icon::icon(icon.clone()).size(icon_size).into(),
        None => widget::space::horizontal()
            .width(Length::Fixed(icon_size as f32))
            .into(),
    };
    let meta = widget::column::with_children(vec![
        widget::text::body(name)
            .font(cosmic::font::semibold())
            .wrapping(cosmic::iced::core::text::Wrapping::None)
            .into(),
        widget::text::body(summary)
            .size(12)
            .class(crate::glass::secondary_text())
            .wrapping(cosmic::iced::core::text::Wrapping::None)
            .into(),
    ])
    .spacing(2)
    .width(Length::Fill)
    .clip(true);
    let row = widget::row::with_capacity(2 + actions.len())
        .push(icon)
        .push(meta)
        .extend(actions)
        .spacing(11)
        .align_y(Alignment::Center);
    widget::column::with_children(vec![
        widget::container(row)
            .height(Length::Fixed(61.0))
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .into(),
        crate::glass::hairline(),
    ])
    .width(Length::Fixed(width as f32))
    .into()
}

pub fn package_card_view<'a>(
    info: &'a AppInfo,
    icon_opt: Option<&'a widget::icon::Handle>,
    controls: Vec<Element<'a, Message>>,
    top_controls: Option<Vec<Element<'a, Message>>>,
    _spacing: &cosmic_theme::Spacing,
    width: usize,
) -> Element<'a, Message> {
    let mut actions = top_controls.unwrap_or_default();
    actions.extend(controls);
    app_row(&info.name, &info.summary, icon_opt, 42, actions, width)
}


/// Section header with a hairline beneath it.
pub fn section_heading<'a>(
    title: String,
    trailing: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    widget::column::with_children(vec![
        widget::row::with_capacity(2 + trailing.len())
            .push(widget::text::body(title).size(15).font(cosmic::font::semibold()))
            .push(widget::space::horizontal().width(Length::Fill))
            .extend(trailing)
            .spacing(10)
            .align_y(Alignment::Center)
            .padding([0, 0, 7, 0])
            .into(),
        crate::glass::hairline(),
    ])
    .into()
}

pub fn see_all<'a>(on_press: Message) -> Element<'a, Message> {
    widget::button::custom(
        widget::row::with_children(vec![
            widget::text::body(fl!("see-all")).into(),
            widget::icon::from_name("go-next-symbolic").size(11).into(),
        ])
        .spacing(2)
        .align_y(Alignment::Center),
    )
    .padding(0)
    .class(crate::glass::link())
    .on_press(on_press)
    .into()
}

pub fn back_button<'a>(label: String, on_press: Message) -> Element<'a, Message> {
    widget::button::custom(
        widget::row::with_children(vec![
            widget::icon::from_name("go-previous-symbolic").size(14).into(),
            widget::text::body(label).into(),
        ])
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding([4, 6])
    .class(crate::glass::tool())
    .on_press(on_press)
    .into()
}

impl Package {
    pub fn grid_metrics(_spacing: &cosmic_theme::Spacing, width: usize) -> GridMetrics {
        GridMetrics::new(width, 260, 28)
    }

    pub fn card_view<'a>(
        &'a self,
        controls: Vec<Element<'a, Message>>,
        top_controls: Option<Vec<Element<'a, Message>>>,
        spacing: &cosmic_theme::Spacing,
        width: usize,
    ) -> Element<'a, Message> {
        package_card_view(
            &self.info,
            Some(&self.icon),
            controls,
            top_controls,
            spacing,
            width,
        )
    }
}
impl App {
    fn selected_buttons(
        &self,
        selected_backend_name: BackendName,
        selected_id: &AppId,
        selected_info: &Arc<AppInfo>,
        addon: bool,
    ) -> Vec<Element<'_, Message>> {
        //TODO: more efficient checks
        let mut waiting_refresh = false;
        for (backend_name, source_id, package_id) in self
            .waiting_installed
            .iter()
            .chain(self.waiting_updates.iter())
        {
            if *backend_name == selected_backend_name
                && source_id == &selected_info.source_id
                && package_id == selected_id
            {
                waiting_refresh = true;
                break;
            }
        }
        let is_installed = self.is_installed(selected_backend_name, selected_id, selected_info);
        let applet_provide = AppProvide::Id("com.yuos.Applet".to_string());
        let mut update_opt = None;
        if let Some(updates) = &self.updates {
            for (backend_name, package) in updates {
                if *backend_name == selected_backend_name
                    && package.info.source_id == selected_info.source_id
                    && &package.id == selected_id
                {
                    update_opt = Some(Message::Operation(
                        OperationKind::Update,
                        *backend_name,
                        package.id.clone(),
                        package.info.clone(),
                    ));
                    break;
                }
            }
        }
        let mut progress_opt = None;
        for (_id, (op, progress)) in self.pending_operations.iter() {
            if op.backend_name == selected_backend_name
                && op
                    .infos
                    .iter()
                    .any(|info| info.source_id == selected_info.source_id)
                && op
                    .package_ids
                    .iter()
                    .any(|package_id| package_id == selected_id)
            {
                progress_opt = Some(*progress);
                break;
            }
        }

        let primary = if addon { Pill::Accent } else { Pill::Solid };
        let mut buttons = Vec::with_capacity(2);
        if let Some(progress) = progress_opt {
            //TODO: get height from theme?
            buttons.push(
                widget::determinate_linear(progress)
                    .width(Length::Fill)
                    .girth(Length::Fixed(4.0))
                    .into(),
            )
        } else if waiting_refresh {
            // Do not show buttons while waiting for refresh
        } else if is_installed {
            //TODO: what if there are multiple desktop IDs?
            if let Some(desktop_id) = selected_info.desktop_ids.first() {
                if selected_info.provides.contains(&applet_provide) {
                    buttons.push(
                        crate::glass::pill_button(fl!("place-on-desktop"), primary, Some(Message::DialogPage(DialogPage::Place(selected_id.clone())))),
                    );
                } else {
                    buttons.push(
                        crate::glass::pill_button(fl!("open"), primary, Some(Message::OpenDesktopId(desktop_id.clone()))),
                    );
                }
            }
            if let Some(update) = update_opt {
                buttons.push(
                    crate::glass::pill_button(fl!("update"), Pill::Accent, Some(update)),
                );
            }
            if !selected_id.is_system() {
                buttons.push(
                    crate::glass::pill_button(fl!("uninstall"), Pill::Danger, Some(Message::DialogPage(DialogPage::Uninstall(
                        selected_backend_name,
                        selected_id.clone(),
                        selected_info.clone(),
                    )))),
                );
            }
        } else {
            buttons.push(
                crate::glass::pill_button(fl!("install"), primary, Some(Message::Operation(
                    OperationKind::Install,
                    selected_backend_name,
                    selected_id.clone(),
                    selected_info.clone(),
                ))),
            )
        }

        buttons
    }

    pub fn selected_sources(
        &self,
        backend_name: BackendName,
        id: &AppId,
        info: &AppInfo,
    ) -> Vec<SelectedSource> {
        let mut sources = Vec::new();
        match self.apps.get(id) {
            Some(infos) => {
                for AppEntry {
                    backend_name,
                    info,
                    installed,
                } in infos.iter()
                {
                    sources.push(SelectedSource::new(*backend_name, info, *installed));
                }
            }
            None => {
                //TODO: warning?
                let installed = self.is_installed(backend_name, id, info);
                sources.push(SelectedSource::new(backend_name, info, installed));
            }
        }
        sources
    }

    pub fn selected_addons(
        &self,
        backend_name: BackendName,
        id: &AppId,
        info: &AppInfo,
    ) -> Vec<(AppId, Arc<AppInfo>)> {
        let mut addons = Vec::new();
        if let Some(backend) = self.backends.get(&backend_name) {
            for appstream_cache in backend.info_caches() {
                if appstream_cache.source_id == info.source_id
                    && let Some(ids) = appstream_cache.addons.get(id)
                {
                    for id in ids {
                        if let Some(info) = appstream_cache.infos.get(id) {
                            addons.push((id.clone(), info.clone()));
                        }
                    }
                }
            }
        }
        addons.par_sort_unstable_by(|a, b| {
            match b.1.monthly_downloads.cmp(&a.1.monthly_downloads) {
                cmp::Ordering::Equal => LANGUAGE_SORTER.compare(&a.1.name, &b.1.name),
                ordering => ordering,
            }
        });
        addons
    }

    pub fn settings(&self) -> Element<'_, Message> {
        let app_theme_selected = match self.config.app_theme {
            AppTheme::Dark => 1,
            AppTheme::Light => 2,
            AppTheme::System => 0,
        };
        widget::settings::view_column(vec![
            widget::settings::section()
                .title(fl!("appearance"))
                .add(
                    widget::settings::item::builder(fl!("theme")).control(widget::dropdown(
                        &self.app_themes,
                        Some(app_theme_selected),
                        move |index| {
                            Message::AppTheme(match index {
                                1 => AppTheme::Dark,
                                2 => AppTheme::Light,
                                _ => AppTheme::System,
                            })
                        },
                    )),
                )
                .into(),
        ])
        .into()
    }

    /// Trailing pill for a result row: Install, Open or Place.
    pub fn result_action<'a>(&'a self, result: &'a SearchResult) -> Option<Element<'a, Message>> {
        let pending = self.pending_operations.values().any(|(op, _)| {
            op.backend_name == result.backend_name
                && op.package_ids.iter().any(|id| id == &result.id)
        });
        if pending {
            return Some(
                widget::text::body(fl!("pending"))
                    .size(12)
                    .class(crate::glass::secondary_text())
                    .into(),
            );
        }
        if self.is_installed(result.backend_name, &result.id, &result.info) {
            let desktop_id = result.info.desktop_ids.first()?;
            let applet = result
                .info
                .provides
                .contains(&AppProvide::Id("com.yuos.Applet".to_string()));
            Some(if applet {
                crate::glass::pill_button(
                    fl!("place-on-desktop"),
                    Pill::Accent,
                    Some(Message::DialogPage(DialogPage::Place(result.id.clone()))),
                )
            } else {
                crate::glass::pill_button(
                    fl!("open"),
                    Pill::Accent,
                    Some(Message::OpenDesktopId(desktop_id.clone())),
                )
            })
        } else {
            Some(crate::glass::pill_button(
                fl!("install"),
                Pill::Accent,
                Some(Message::Operation(
                    OperationKind::Install,
                    result.backend_name,
                    result.id.clone(),
                    result.info.clone(),
                )),
            ))
        }
    }

    fn results_grid<'a, F: Fn(usize) -> Message + 'a>(
        &'a self,
        results: &'a [SearchResult],
        spacing: cosmic_theme::Spacing,
        width: usize,
        callback: F,
    ) -> Element<'a, Message> {
        SearchResult::grid_view(results, spacing, width, callback, |result| {
            self.result_action(result)
        })
    }

    /// Page title shown in the window toolbar.
    pub fn page_title(&self) -> String {
        if let Some(selected) = &self.selected_opt {
            return selected.info.name.clone();
        }
        if let Some((input, _)) = &self.search_results {
            return format!("{}: {}", fl!("search"), input);
        }
        let nav_page = self
            .nav_model
            .active_data::<NavPage>()
            .copied()
            .unwrap_or_default();
        match (nav_page, self.explore_page_opt) {
            (NavPage::Explore, Some(page)) => page.title(),
            _ => nav_page.title(),
        }
    }

    pub fn view_responsive(&self, size: Size) -> Element<'_, Message> {
        self.size.set(Some(size));
        let spacing = theme::active().cosmic().spacing;
        let grid_width = (size.width - 56.0).floor().max(0.0) as usize;
        if let Some(selected) = &self.selected_opt {
            return page(self.detail_view(selected, grid_width));
        }
        if let Some((input, results)) = &self.search_results {
            let results_len = cmp::min(results.len(), MAX_RESULTS);
            let caption = if results.is_empty() {
                fl!("no-results", search = input.as_str())
            } else {
                fl!(
                    "search-results",
                    count = results.len(),
                    search = input.as_str()
                )
            };
            return page(vec![
                widget::text::body(caption)
                    .size(12)
                    .class(crate::glass::secondary_text())
                    .apply(widget::container)
                    .padding([14, 0, 4, 0])
                    .into(),
                self.results_grid(
                    &results[..results_len],
                    spacing,
                    grid_width,
                    Message::SelectSearchResult,
                ),
                ask_yu_search_row(input),
            ]);
        }
        let nav_page = self
            .nav_model
            .active_data::<NavPage>()
            .copied()
            .unwrap_or_default();
        match nav_page {
            NavPage::YuOsApps => {
                let mut children = vec![widget::row::with_children(vec![
                    widget::space::horizontal().width(Length::Fill).into(),
                    self.source_check_button(),
                ])
                .padding([14, 0, 2, 0])
                .into()];
                if let Some(error) = &self.refresh_error {
                    children.push(note(fl!("yuos-refresh-failed", error = error.as_str())));
                }
                children.push(self.yuos_apps_view(false));
                page(children)
            }
            NavPage::Explore => match self.explore_page_opt {
                Some(explore_page) => {
                    let mut children = vec![
                        widget::container(back_button(
                            NavPage::Explore.title(),
                            Message::ExplorePage(None),
                        ))
                        .padding([10, 0, 0, 0])
                        .into(),
                    ];
                    if let Some(results) = self.explore_results.get(&explore_page) {
                        let results_len = cmp::min(results.len(), MAX_RESULTS);
                        children.push(self.results_grid(
                            &results[..results_len],
                            spacing,
                            grid_width,
                            move |result_i| Message::SelectExploreResult(explore_page, result_i),
                        ));
                    }
                    page(children)
                }
                None => page(self.explore_home(spacing, grid_width)),
            },
            NavPage::Installed => {
                let mut children = vec![widget::space::vertical().height(14).into()];
                if let Some(installed) = &self.installed_results {
                    if installed.is_empty() {
                        children.push(note(fl!("no-installed-applications")));
                    }
                    let GridMetrics {
                        cols,
                        item_width,
                        column_spacing,
                    } = Package::grid_metrics(&spacing, grid_width);
                    let mut grid = widget::grid();
                    let mut col = 0;
                    for (installed_i, result) in installed.iter().enumerate() {
                        if col >= cols {
                            grid = grid.insert_row();
                            col = 0;
                        }
                        let actions = result
                            .info
                            .desktop_ids
                            .first()
                            .map(|desktop_id| {
                                crate::glass::pill_button(
                                    fl!("open"),
                                    Pill::Accent,
                                    Some(Message::OpenDesktopId(desktop_id.clone())),
                                )
                            })
                            .into_iter()
                            .collect();
                        grid = grid.push(
                            widget::mouse_area(package_card_view(
                                &result.info,
                                result.icon_opt.as_ref(),
                                actions,
                                None,
                                &spacing,
                                item_width,
                            ))
                            .on_press(Message::SelectInstalled(installed_i)),
                        );
                        col += 1;
                    }
                    children.push(grid.column_spacing(column_spacing).row_spacing(0).into());
                }
                page(children)
            }
            NavPage::Updates => page(self.updates_view()),
            nav_page => {
                let mut children = vec![widget::space::vertical().height(14).into()];
                if matches!(nav_page, NavPage::Applets) {
                    let sources = self.sources();
                    if !sources.is_empty()
                        && sources.iter().any(|source| {
                            matches!(source.kind, SourceKind::Recommended { enabled: false, .. })
                        })
                    {
                        children.push(
                            widget::row::with_children(vec![
                                widget::icon::from_name("dialog-warning-symbolic")
                                    .size(14)
                                    .into(),
                                widget::text::body(fl!("enable-applet-sources"))
                                    .size(12)
                                    .class(crate::glass::secondary_text())
                                    .into(),
                                widget::button::custom(
                                    widget::text::body(fl!("manage-repositories")).size(12),
                                )
                                .padding(0)
                                .class(crate::glass::link())
                                .on_press(Message::ToggleContextPage(ContextPage::Repositories))
                                .into(),
                            ])
                            .spacing(7)
                            .align_y(Alignment::Center)
                            .padding([8, 0])
                            .into(),
                        );
                    }
                }
                if let Some((_, results)) = &self.category_results {
                    let results_len = cmp::min(results.len(), MAX_RESULTS);
                    children.push(self.results_grid(
                        &results[..results_len],
                        spacing,
                        grid_width,
                        Message::SelectCategoryResult,
                    ));
                }
                page(children)
            }
        }
    }

    fn explore_home(
        &self,
        spacing: cosmic_theme::Spacing,
        grid_width: usize,
    ) -> Vec<Element<'_, Message>> {
        let mut children = Vec::new();
        let mut banners = Vec::with_capacity(2);
        for (explore_page, purple) in [
            (ExplorePage::EditorsChoice, false),
            (ExplorePage::MadeForYuOs, true),
        ] {
            if let Some(result) = self
                .explore_results
                .get(&explore_page)
                .and_then(|results| results.first())
            {
                banners.push(self.featured_banner(explore_page, result, purple));
            }
        }
        if !banners.is_empty() {
            let banners: Element<_> = if grid_width >= 640 {
                widget::row::with_children(banners).spacing(20).into()
            } else {
                widget::column::with_children(banners).spacing(14).into()
            };
            children.push(widget::container(banners).padding([12, 0, 0, 0]).into());
        }

        for explore_page in ExplorePage::all() {
            let Some(results) = self.explore_results.get(explore_page) else {
                continue;
            };
            if results.is_empty() {
                continue;
            }
            let GridMetrics { cols, .. } = SearchResult::grid_metrics(&spacing, grid_width);
            let results_len = cmp::min(results.len(), cols.max(2) * 2);
            let explore_page = *explore_page;
            children.push(
                widget::column::with_children(vec![
                    section_heading(
                        explore_page.title(),
                        vec![see_all(Message::ExplorePage(Some(explore_page)))],
                    ),
                    self.results_grid(
                        &results[..results_len],
                        spacing,
                        grid_width,
                        move |result_i| Message::SelectExploreResult(explore_page, result_i),
                    ),
                ])
                .padding([22, 0, 0, 0])
                .into(),
            );
        }
        children
    }

    fn featured_banner<'a>(
        &'a self,
        explore_page: ExplorePage,
        result: &'a SearchResult,
        purple: bool,
    ) -> Element<'a, Message> {
        let icon: Element<_> = match &result.icon_opt {
            Some(icon) => widget::icon::icon(icon.clone()).size(72).into(),
            None => widget::space::horizontal().width(Length::Fixed(72.0)).into(),
        };
        let eyebrow = widget::text::body(explore_page.title().to_uppercase())
            .size(11)
            .font(cosmic::font::semibold())
            .class(if purple {
                crate::glass::purple_text()
            } else {
                crate::glass::accent_text()
            });
        let mut info = widget::column::with_capacity(5)
            .push(eyebrow)
            .push(
                widget::text::body(&result.info.name)
                    .size(15)
                    .font(cosmic::font::semibold()),
            )
            .push(widget::text::body(&result.info.summary).class(crate::glass::secondary_text()))
            .spacing(5)
            .width(Length::Fill);
        if let Some(action) = self.result_action(result) {
            info = info
                .push(widget::space::vertical().height(Length::Fixed(4.0)))
                .push(action);
        }
        widget::mouse_area(
            widget::container(
                widget::row::with_children(vec![icon, info.into()])
                    .spacing(18)
                    .align_y(Alignment::Center),
            )
            .padding([22, 24])
            .height(Length::Fixed(164.0))
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .class(crate::glass::banner_class(purple)),
        )
        .on_press(Message::SelectExploreResult(explore_page, 0))
        .into()
    }

    fn detail_view<'a>(
        &'a self,
        selected: &'a crate::Selected,
        grid_width: usize,
    ) -> Vec<Element<'a, Message>> {
        let mut children: Vec<Element<'a, Message>> = Vec::new();
        children.push(
            widget::container(back_button(fl!("back"), Message::SelectNone))
                .padding([10, 0, 0, 0])
                .into(),
        );

        let icon: Element<_> = match &selected.icon_opt {
            Some(icon) => widget::icon::icon(icon.clone()).size(72).into(),
            None => widget::space::horizontal().width(Length::Fixed(72.0)).into(),
        };
        let developer = if selected.info.developer_name.is_empty() {
            fl!("app-developers", app = selected.info.name.as_str())
        } else {
            selected.info.developer_name.clone()
        };
        let homepage = selected.info.urls.iter().find_map(|url| match url {
            AppUrl::Homepage(url) => Some(url.clone()),
            _ => None,
        });
        let buttons =
            self.selected_buttons(selected.backend_name, &selected.id, &selected.info, false);
        children.push(
            widget::row::with_children(vec![
                icon,
                widget::column::with_children(vec![
                    widget::text::body(&selected.info.name)
                        .size(15)
                        .font(cosmic::font::semibold())
                        .into(),
                    widget::text::body(&selected.info.summary)
                        .class(crate::glass::secondary_text())
                        .into(),
                    widget::button::custom(widget::text::body(developer.clone()).size(12))
                        .padding(0)
                        .class(crate::glass::link())
                        .on_press_maybe(homepage.map(Message::LaunchUrl))
                        .into(),
                ])
                .spacing(3)
                .width(Length::Fill)
                .into(),
                widget::row::with_children(buttons)
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .into(),
            ])
            .spacing(18)
            .align_y(Alignment::Center)
            .padding([18, 0, 16, 0])
            .into(),
        );

        // Info strip
        let selected_source = selected.sources.iter().position(|source| {
            source.backend_name == selected.backend_name
                && source.source_id == selected.info.source_id
        });
        let source_value: Element<_> = if selected.sources.len() == 1 {
            strip_value(selected.sources[0].as_ref().to_string())
        } else {
            widget::dropdown(&selected.sources, selected_source, Message::SelectedSource).into()
        };
        let mut cells = vec![
            strip_cell(fl!("source"), source_value),
            strip_cell(fl!("developer"), strip_value(developer)),
        ];
        if selected.info.monthly_downloads > 0 {
            cells.push(strip_cell(
                fl!("monthly-downloads"),
                strip_value(selected.info.monthly_downloads.to_string()),
            ));
        }
        if let Some(release) = selected.info.releases.first() {
            cells.push(strip_cell(
                fl!("version-label"),
                strip_value(release.version.clone()),
            ));
        }
        if let Some(license) = &selected.info.license_opt {
            cells.push(strip_cell(fl!("licenses"), strip_value(license.clone())));
        }
        let strip: Element<_> = if grid_width >= 560 {
            let mut row = widget::row::with_capacity(cells.len() * 2);
            for (i, cell) in cells.into_iter().enumerate() {
                if i > 0 {
                    row = row.push(
                        widget::container(
                            cosmic::iced::widget::rule::vertical(1).class(crate::glass::rule()),
                        )
                        .height(Length::Fixed(36.0)),
                    );
                }
                row = row.push(cell);
            }
            row.align_y(Alignment::Center).padding([10, 0]).into()
        } else {
            widget::column::with_children(cells)
                .spacing(10)
                .padding([10, 0])
                .into()
        };
        children.push(crate::glass::hairline());
        children.push(strip);
        children.push(crate::glass::hairline());

        // Screenshots
        if let Some(screenshot) = selected.info.screenshots.get(selected.screenshot_shown) {
            let count = selected.info.screenshots.len();
            let prev = selected
                .screenshot_shown
                .checked_sub(1)
                .unwrap_or_else(|| count.saturating_sub(1));
            let next = if selected.screenshot_shown + 1 == count {
                0
            } else {
                selected.screenshot_shown + 1
            };
            let nav_button = |name: &'static str, index: usize| -> Element<'a, Message> {
                widget::button::custom(widget::icon::from_name(name).size(16))
                    .padding(6)
                    .class(crate::glass::tool())
                    .on_press_maybe(
                        (index != selected.screenshot_shown)
                            .then_some(Message::SelectedScreenshotShown(index)),
                    )
                    .into()
            };
            let image_height = Length::Fixed(280.0);
            let image: Element<_> = match selected.screenshot_images.get(&selected.screenshot_shown)
            {
                Some(image) => widget::container(widget::image(image.clone()))
                    .center_x(Length::Fill)
                    .center_y(image_height)
                    .padding(4)
                    .class(crate::glass::screenshot_class())
                    .into(),
                None => widget::container(widget::space::horizontal())
                    .width(Length::Fill)
                    .height(image_height)
                    .class(crate::glass::screenshot_class())
                    .into(),
            };
            let mut dots = widget::row::with_capacity(count).spacing(6);
            for i in 0..count {
                let on = i == selected.screenshot_shown;
                dots = dots.push(
                    widget::container(widget::space::horizontal().width(Length::Fixed(6.0)))
                        .width(Length::Fixed(6.0))
                        .height(Length::Fixed(6.0))
                        .class(theme::Container::custom(move |theme| {
                            let mut color = crate::glass::secondary(theme);
                            if !on {
                                color.a = 0.3;
                            }
                            widget::container::Style {
                                background: Some(cosmic::iced::Background::Color(color)),
                                border: cosmic::iced::Border {
                                    radius: 3.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        })),
                );
            }
            children.push(
                widget::column::with_children(vec![
                    widget::row::with_children(vec![
                        nav_button("go-previous-symbolic", prev),
                        image,
                        nav_button("go-next-symbolic", next),
                    ])
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .into(),
                    widget::text::body(&screenshot.caption)
                        .size(12)
                        .class(crate::glass::secondary_text())
                        .into(),
                    dots.into(),
                ])
                .spacing(8)
                .align_x(Alignment::Center)
                .padding([18, 0, 0, 0])
                .into(),
            );
        }

        // Main column: About, latest version, addons
        let mut main = Vec::new();
        main.push(block(
            fl!("about"),
            vec![
                widget::text::body(if selected.info.description.is_empty() {
                    fl!("no-description")
                } else {
                    selected.info.description.clone()
                })
                .into(),
            ],
        ));
        if let Some(release) = selected.info.releases.first() {
            let mut lines: Vec<Element<_>> = vec![
                widget::text::body(fl!("version", version = release.version.as_str()))
                    .font(cosmic::font::semibold())
                    .into(),
            ];
            if let Some(timestamp) = release.timestamp
                && let Some(utc) = chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0)
            {
                let local = chrono::DateTime::<chrono::Local>::from(utc);
                lines.push(
                    widget::text::body(format!("{}", local.format("%b %-d, %-Y")))
                        .size(12)
                        .class(crate::glass::secondary_text())
                        .into(),
                );
            }
            if let Some(description) = &release.description {
                lines.push(widget::text::body(description).into());
            }
            main.push(block(fl!("latest-version"), lines));
        }
        if !selected.addons.is_empty() {
            let take = if selected.addons_view_more {
                selected.addons.len()
            } else {
                4
            };
            let mut lines: Vec<Element<_>> = Vec::new();
            for (addon_id, addon_info) in selected.addons.iter().take(take) {
                let buttons =
                    self.selected_buttons(selected.backend_name, addon_id, addon_info, true);
                lines.push(app_row(
                    &addon_info.name,
                    &addon_info.summary,
                    None,
                    0,
                    buttons,
                    grid_width.min(640),
                ));
            }
            if selected.addons.len() > 4 && !selected.addons_view_more {
                lines.push(
                    widget::button::custom(widget::text::body(fl!("view-more")))
                        .padding([6, 0])
                        .class(crate::glass::link())
                        .on_press(Message::SelectedAddonsViewMore(true))
                        .into(),
                );
            }
            main.push(block(fl!("addons"), lines));
        }

        // Side column: licenses and links
        let mut side = Vec::new();
        if let Some(license) = &selected.info.license_opt {
            let mut lines: Vec<Element<_>> = Vec::new();
            match spdx::Expression::parse_mode(license, spdx::ParseMode::LAX) {
                Ok(expr) => {
                    for item in expr.requirements() {
                        match &item.req.license {
                            spdx::LicenseItem::Spdx { id, .. } => {
                                lines.push(widget::text::body(id.full_name).into());
                            }
                            spdx::LicenseItem::Other { lic_ref, .. } => {
                                let url = lic_ref.splitn(2, '=').nth(1).map(str::to_string);
                                lines.push(
                                    widget::button::custom(widget::text::body(fl!("proprietary")))
                                        .padding(0)
                                        .class(crate::glass::link())
                                        .on_press_maybe(url.map(Message::LaunchUrl))
                                        .into(),
                                );
                            }
                        }
                    }
                }
                Err(_) => lines.push(widget::text::body(license).into()),
            }
            side.push(block(fl!("licenses"), lines));
        }
        if !selected.info.urls.is_empty() {
            let mut lines: Vec<Element<_>> = Vec::new();
            for app_url in &selected.info.urls {
                let (name, url) = match app_url {
                    AppUrl::BugTracker(url) => (fl!("bug-tracker"), url),
                    AppUrl::Contact(url) => (fl!("contact"), url),
                    AppUrl::Donation(url) => (fl!("donation"), url),
                    AppUrl::Faq(url) => (fl!("faq"), url),
                    AppUrl::Help(url) => (fl!("help"), url),
                    AppUrl::Homepage(url) => (fl!("homepage"), url),
                    AppUrl::Translate(url) => (fl!("translate"), url),
                };
                lines.push(
                    widget::button::custom(
                        widget::row::with_children(vec![
                            widget::text::body(name).into(),
                            widget::space::horizontal().width(Length::Fill).into(),
                            widget::icon::from_name("go-next-symbolic").size(11).into(),
                        ])
                        .align_y(Alignment::Center),
                    )
                    .width(Length::Fill)
                    .padding([6, 0])
                    .class(crate::glass::link())
                    .on_press(Message::LaunchUrl(url.to_string()))
                    .into(),
                );
            }
            side.push(block(fl!("links"), lines));
        }

        let main = widget::column::with_children(main)
            .spacing(16)
            .width(Length::Fill);
        let side = widget::column::with_children(side).spacing(16);
        children.push(if grid_width >= 720 {
            widget::row::with_children(vec![main.into(), side.width(Length::Fixed(260.0)).into()])
                .spacing(36)
                .padding([20, 0, 0, 0])
                .into()
        } else {
            widget::column::with_children(vec![main.into(), side.width(Length::Fill).into()])
                .spacing(16)
                .padding([20, 0, 0, 0])
                .into()
        });
        children
    }

    fn updates_view(&self) -> Vec<Element<'_, Message>> {
        let mut children = Vec::new();
        children.push(
            widget::container(section_heading(
                NavPage::YuOsApps.title(),
                vec![self.source_check_button()],
            ))
            .padding([12, 0, 0, 0])
            .into(),
        );
        let checked = self
            .yuos_catalog
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .and_then(|catalog| catalog.checked_at.as_deref());
        children.push(note(match checked {
            Some(time) => fl!("yuos-last-checked", time = time),
            None => fl!("yuos-not-checked"),
        }));
        if let Some(error) = &self.refresh_error {
            children.push(note(fl!("yuos-refresh-failed", error = error.as_str())));
        }
        children.push(self.yuos_apps_view(true));

        let update_all = self
            .updates
            .as_ref()
            .is_some_and(|updates| !updates.is_empty())
            .then(|| {
                crate::glass::pill_button(fl!("update-all"), Pill::Solid, Some(Message::UpdateAll))
            });
        children.push(
            widget::container(section_heading(
                fl!("linux-system-updates"),
                update_all.into_iter().collect(),
            ))
            .padding([24, 0, 0, 0])
            .into(),
        );
        for (source, error) in &self.update_errors {
            children.push(note(fl!(
                "yuos-update-query-failed",
                source = source.as_str(),
                error = error.as_str()
            )));
        }
        let Some(updates) = &self.updates else {
            children.push(note(fl!("checking-for-updates")));
            return children;
        };
        if updates.is_empty()
            && self.update_errors.is_empty()
            && self.pending_backend_updates.is_empty()
            && !self.fetching_backends
            && !self.refreshing_sources
            && self.refresh_error.is_none()
        {
            children.push(note(fl!("no-updates")));
        }
        let width = self
            .size
            .get()
            .map_or(800, |size| (size.width - 56.0).max(0.0) as usize);
        for (updates_i, (backend_name, package)) in updates.iter().enumerate() {
            let waiting_refresh = self
                .waiting_installed
                .iter()
                .chain(self.waiting_updates.iter())
                .any(|(other, source_id, package_id)| {
                    other == backend_name
                        && source_id == &package.info.source_id
                        && package_id == &package.id
                });
            let progress_opt = self.pending_operations.values().find_map(|(op, progress)| {
                (&op.backend_name == backend_name
                    && op
                        .infos
                        .iter()
                        .any(|info| info.source_id == package.info.source_id)
                    && op.package_ids.iter().any(|id| id == &package.id))
                    .then_some(*progress)
            });
            let mut actions: Vec<Element<_>> = vec![
                widget::text::body(package.version.as_str())
                    .size(12)
                    .class(crate::glass::tertiary_text())
                    .into(),
                widget::button::custom(widget::text::body(fl!("release-notes")).size(12))
                    .padding([0, 4])
                    .class(crate::glass::link())
                    .on_press(Message::ToggleContextPage(ContextPage::ReleaseNotes(
                        updates_i,
                        package.info.name.clone(),
                    )))
                    .into(),
            ];
            if let Some(progress) = progress_opt {
                actions.push(
                    widget::determinate_linear(progress)
                        .width(Length::Fixed(80.0))
                        .girth(Length::Fixed(4.0))
                        .into(),
                );
            } else if !waiting_refresh {
                actions.push(crate::glass::pill_button(
                    fl!("update"),
                    Pill::Accent,
                    Some(Message::Operation(
                        OperationKind::Update,
                        *backend_name,
                        package.id.clone(),
                        package.info.clone(),
                    )),
                ));
            }
            children.push(
                widget::mouse_area(package.card_view(actions, None, &theme::spacing(), width))
                    .on_press(Message::SelectUpdates(updates_i))
                    .into(),
            );
        }
        children
    }
}

fn page<'a>(children: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    widget::column::with_children(children)
        .padding([4, 28, 28, 28])
        .width(Length::Fill)
        .into()
}

fn note<'a>(text: String) -> Element<'a, Message> {
    widget::text::body(text)
        .size(12)
        .class(crate::glass::secondary_text())
        .apply(widget::container)
        .padding([8, 0])
        .into()
}

fn block<'a>(title: String, lines: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    widget::column::with_children(vec![
        widget::text::body(title)
            .size(15)
            .font(cosmic::font::semibold())
            .into(),
        widget::column::with_children(lines).spacing(6).into(),
    ])
    .spacing(8)
    .into()
}

fn strip_value<'a>(value: String) -> Element<'a, Message> {
    widget::text::body(value)
        .font(cosmic::font::semibold())
        .class(crate::glass::secondary_text())
        .wrapping(cosmic::iced::core::text::Wrapping::None)
        .into()
}

fn strip_cell<'a>(key: String, value: Element<'a, Message>) -> Element<'a, Message> {
    widget::column::with_children(vec![
        widget::text::body(key.to_uppercase())
            .size(10.5)
            .font(cosmic::font::semibold())
            .class(crate::glass::tertiary_text())
            .wrapping(cosmic::iced::core::text::Wrapping::None)
            .into(),
        value,
    ])
    .spacing(5)
    .align_x(Alignment::Center)
    .width(Length::Fill)
    .clip(true)
    .into()
}

/// Inline Ask Yu row under search results.
fn ask_yu_search_row<'a>(query: &str) -> Element<'a, Message> {
    let q = query.trim();
    let label = if q.is_empty() {
        fl!("ai-search-ask-yu")
    } else {
        fl!("ai-search-ask-yu-q", query = q)
    };
    let tile = widget::container(widget::icon::from_name("chat-symbolic").size(16))
        .center_x(Length::Fixed(30.0))
        .center_y(Length::Fixed(30.0))
        .class(theme::Container::custom(crate::glass::ask_tile));
    let row = widget::row::with_children(vec![
        tile.into(),
        widget::column::with_children(vec![
            widget::text::body(label).font(cosmic::font::semibold()).into(),
            widget::text::body(fl!("ai-search-ask-yu-hint"))
                .size(12)
                .class(crate::glass::secondary_text())
                .into(),
        ])
        .spacing(2)
        .width(Length::Fill)
        .into(),
        crate::glass::pill_button(fl!("ask-yu-short"), Pill::Accent, Some(Message::AskYuSearch)),
    ])
    .spacing(12)
    .align_y(Alignment::Center);
    widget::column::with_children(vec![
        widget::mouse_area(
            widget::container(row)
                .height(Length::Fixed(61.0))
                .align_y(Alignment::Center),
        )
        .on_press(Message::AskYuSearch)
        .into(),
        crate::glass::hairline(),
    ])
    .padding([8, 0, 0, 0])
    .into()
}
