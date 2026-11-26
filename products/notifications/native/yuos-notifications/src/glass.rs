// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

//! Theme-aware surfaces for desktop notification banners.
//!
//! Note: the toolkit's stacked `cards` widget draws no per-card background in
//! the expanded (multi-toast) state, so wrapping each notification's inner
//! element in [`card_class`] makes our frosted surface the visible toast while
//! preserving the widget's activation / dismiss behavior.

use cosmic::{
    iced::{Background, Border, Color, Shadow, Vector, widget::container},
    theme,
};

/// Neutral hairline for glass chrome.
///
/// `on_bg_color` is near-black on a light theme and near-white on a dark one,
/// so a low alpha of it separates the card from the wallpaper in both without
/// tinting the glass.
fn hairline(theme: &cosmic::Theme) -> Color {
    let cosmic = theme.cosmic();
    let mut color: Color = cosmic.on_bg_color().into();
    color.a = if cosmic.is_dark { 0.15 } else { 0.09 };
    color
}

pub fn card_class() -> theme::Container<'static> {
    theme::Container::custom(|theme| {
        let cosmic = theme.cosmic();

        let mut background: Color = cosmic.bg_component_color().into();
        background.a = 0.86;

        let on: Color = cosmic.on_bg_color().into();

        container::Style {
            icon_color: Some(on),
            text_color: Some(on),
            background: Some(Background::Color(background)),
            border: Border {
                radius: 14.0.into(),
                width: 1.0,
                color: hairline(theme),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.10, 0.13),
                offset: Vector::new(0.0, 3.0),
                blur_radius: 10.0,
            },
            snap: true,
        }
    })
}

pub fn icon_class() -> theme::Container<'static> {
    theme::Container::custom(|theme| {
        let cosmic = theme.cosmic();
        container::Style {
            icon_color: Some(cosmic.accent_color().into()),
            background: Some(Background::Color(cosmic.bg_component_color().into())),
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: hairline(theme),
            },
            ..Default::default()
        }
    })
}
