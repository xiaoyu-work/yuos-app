// SPDX-License-Identifier: GPL-3.0-only

use cosmic::widget::RcElementWrapper;
use cosmic::widget::menu::key_bind::KeyBind;
use cosmic::widget::menu::{self, ItemHeight, ItemWidth, MenuBar};
use cosmic::widget::{self, icon};
use cosmic::{Element, Theme, theme};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::{Action, ConfigState, Message, fl};

fn menu_label(label: String) -> RcElementWrapper<Message> {
    RcElementWrapper::new(
        widget::button::custom(
            widget::row::with_children(vec![
                widget::text(label).size(13).into(),
                icon::from_name("go-down-symbolic").size(9).into(),
            ])
            .align_y(cosmic::iced::Alignment::Center)
            .spacing(4),
        )
        .padding([4, 8])
        .class(theme::Button::MenuRoot)
        .into(),
    )
}

fn appearance(theme: &Theme) -> menu::Appearance {
    let cosmic = theme.cosmic();
    menu::Appearance {
        background: crate::style::surface_color(theme),
        border_width: 1.0,
        bar_border_radius: [6.0; 4],
        menu_border_radius: [8.0; 4],
        border_color: cosmic.background.component.divider.into(),
        background_expand: [1; 4],
        path: cosmic.background.component.hover.into(),
    }
}

pub fn menu_bar<'a>(
    config_state: &ConfigState,
    key_binds: &HashMap<KeyBind, Action>,
    projects: &[(String, PathBuf)],
    ab_repeat: &Option<(Option<f64>, Option<f64>)>,
    has_video: bool,
) -> Element<'a, Message> {
    let home_dir_opt = std::env::home_dir();
    let format_path = |path: &PathBuf| -> String {
        if let Some(home_dir) = &home_dir_opt
            && let Ok(part) = path.strip_prefix(home_dir)
        {
            return format!("~/{}", part.display());
        }
        path.display().to_string()
    };
    let format_url = |url: &url::Url| -> String {
        match url.to_file_path() {
            Ok(path) => format_path(&path),
            Err(()) => url.to_string(),
        }
    };

    let files_len = if config_state.recent_files.is_empty() {
        0
    } else {
        config_state.recent_files.len() + 2
    };
    let mut recent_files = Vec::with_capacity(files_len);
    for (i, path) in config_state.recent_files.iter().enumerate() {
        recent_files.push(menu::Item::Button(
            format_url(path),
            None,
            Action::FileOpenRecent(i),
        ));
    }
    if files_len > 0 {
        recent_files.push(menu::Item::Divider);
        recent_files.push(menu::Item::Button(
            fl!("clear-recent"),
            None,
            Action::FileClearRecents,
        ));
    }

    let projects_len = if config_state.recent_projects.is_empty() {
        0
    } else {
        config_state.recent_projects.len() + 2
    };
    let mut recent_projects = Vec::with_capacity(projects_len);
    for (i, path) in config_state.recent_projects.iter().enumerate() {
        recent_projects.push(menu::Item::Button(
            format_path(path),
            None,
            Action::FolderOpenRecent(i),
        ));
    }
    if projects_len > 0 {
        recent_projects.push(menu::Item::Divider);
        recent_projects.push(menu::Item::Button(
            fl!("clear-recent"),
            None,
            Action::FolderClearRecents,
        ));
    }

    let mut close_projects = Vec::with_capacity(projects.len());
    for (folder_i, (name, _path)) in projects.iter().enumerate() {
        close_projects.push(menu::Item::Button(
            name.clone(),
            None,
            Action::FolderClose(folder_i),
        ));
    }

    let mut playback = Vec::with_capacity(3);

    if has_video {
        playback.push(menu::Item::Button(
            fl!("next-frame"),
            Some(icon::from_name("go-next-symbolic").handle()),
            Action::NextFrame,
        ));
        playback.push(menu::Item::Button(
            fl!("previous-frame"),
            Some(icon::from_name("go-previous-symbolic").handle()),
            Action::PreviousFrame,
        ));
    }

    playback.push(menu::Item::Button(
        match ab_repeat {
            None => fl!("ab-repeat-set-a"),
            Some((_, None)) => fl!("ab-repeat-set-b"),
            Some((_, Some(_))) => fl!("ab-repeat-clear"),
        },
        None,
        Action::AbRepeat,
    ));
    MenuBar::new(vec![
        menu::Tree::with_children(
            menu_label(fl!("file")),
            menu::items(
                key_binds,
                vec![
                    menu::Item::Button(
                        fl!("open-media"),
                        Some(icon::from_name("document-open-symbolic").handle()),
                        Action::FileOpen,
                    ),
                    menu::Item::Folder(fl!("open-recent-media"), recent_files),
                    menu::Item::Button(fl!("close-file"), None, Action::FileClose),
                    menu::Item::Divider,
                    menu::Item::Button(
                        fl!("open-media-folder"),
                        Some(icon::from_name("folder-open-symbolic").handle()),
                        Action::FolderOpen,
                    ),
                    menu::Item::Folder(fl!("open-recent-media-folder"), recent_projects),
                    menu::Item::Folder(fl!("close-media-folder"), close_projects),
                    menu::Item::Divider,
                    menu::Item::Button(fl!("quit"), None, Action::WindowClose),
                ],
            ),
        ),
        menu::Tree::with_children(
            menu_label(fl!("playback")),
            menu::items(key_binds, playback),
        ),
    ])
    .item_height(ItemHeight::Dynamic(27))
    .item_width(ItemWidth::Uniform(288))
    .style(appearance as fn(&Theme) -> menu::Appearance)
    .spacing(theme::active().cosmic().spacing.space_xxxs.into())
    .into()
}
