// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

//! Finder-style surfaces for the Store: a light translucent sidebar, plain
//! content background, hairline separators, blue selection and pill actions.

use cosmic::iced::{Background, Border, Color, Gradient, Radians, gradient::Linear};
use cosmic::widget::{button, container};
use cosmic::{Theme, theme};

pub const ACCENT: Color = Color::from_rgb(0x0a as f32 / 255.0, 0x84 as f32 / 255.0, 1.0);
const DANGER: Color = Color::from_rgb(0xe5 as f32 / 255.0, 0x48 as f32 / 255.0, 0x4d as f32 / 255.0);

fn dark(theme: &Theme) -> bool {
    theme.cosmic().is_dark
}

fn fg(theme: &Theme) -> Color {
    if dark(theme) {
        Color::from_rgb8(0xee, 0xf2, 0xf7)
    } else {
        Color::from_rgb8(0x1d, 0x1d, 0x1f)
    }
}

pub fn secondary(theme: &Theme) -> Color {
    if dark(theme) {
        Color::from_rgb8(0x9a, 0xa5, 0xb5)
    } else {
        Color::from_rgb8(0x6e, 0x6e, 0x73)
    }
}

fn tertiary(theme: &Theme) -> Color {
    if dark(theme) {
        Color::from_rgb8(0x7a, 0x84, 0x93)
    } else {
        Color::from_rgb8(0xa1, 0xa1, 0xa6)
    }
}

fn hair(theme: &Theme) -> Color {
    if dark(theme) {
        Color::from_rgba(1.0, 1.0, 1.0, 0.10)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.09)
    }
}

fn wash(theme: &Theme, alpha: f32) -> Color {
    if dark(theme) {
        Color::from_rgba(1.0, 1.0, 1.0, alpha * 1.4)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, alpha)
    }
}

fn surface(bg: Color, fg: Color) -> container::Style {
    container::Style {
        icon_color: Some(fg),
        text_color: Some(fg),
        background: Some(Background::Color(bg)),
        snap: true,
        ..Default::default()
    }
}

pub fn content(theme: &Theme) -> container::Style {
    surface(
        if dark(theme) {
            Color::from_rgb8(0x12, 0x16, 0x1f)
        } else {
            Color::WHITE
        },
        fg(theme),
    )
}

pub fn sidebar(theme: &Theme) -> container::Style {
    surface(
        if dark(theme) {
            Color::from_rgb8(0x18, 0x1d, 0x26)
        } else {
            Color::from_rgb8(0xe4, 0xed, 0xf6)
        },
        fg(theme),
    )
}

pub fn menu(theme: &Theme) -> container::Style {
    let mut style = content(theme);
    style.border = Border {
        color: hair(theme),
        width: 1.0,
        radius: 8.0.into(),
    };
    style.shadow = cosmic::iced::Shadow {
        color: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
        offset: cosmic::iced::Vector::new(0.0, 4.0),
        blur_radius: 14.0,
    };
    style
}

pub fn rule() -> theme::Rule {
    theme::Rule::Custom(Box::new(|theme| cosmic::iced::widget::rule::Style {
        color: hair(theme),
        radius: 0.0.into(),
        fill_mode: cosmic::iced::widget::rule::FillMode::Full,
        snap: true,
    }))
}

pub fn hairline<'a, M: 'a>() -> cosmic::Element<'a, M> {
    cosmic::iced::widget::rule::horizontal(1).class(rule()).into()
}

pub fn secondary_text() -> theme::Text {
    theme::Text::Custom(|t| cosmic::iced::widget::text::Style {
        color: Some(secondary(t)),
    })
}

pub fn tertiary_text() -> theme::Text {
    theme::Text::Custom(|t| cosmic::iced::widget::text::Style {
        color: Some(tertiary(t)),
    })
}

pub fn accent_text() -> theme::Text {
    theme::Text::Custom(|_| cosmic::iced::widget::text::Style {
        color: Some(ACCENT),
    })
}

pub fn purple_text() -> theme::Text {
    theme::Text::Custom(|_| cosmic::iced::widget::text::Style {
        color: Some(Color::from_rgb8(0x7d, 0x5b, 0xdc)),
    })
}

pub fn banner_class(purple: bool) -> theme::Container<'static> {
    theme::Container::custom(move |theme| {
        let tint = match (dark(theme), purple) {
            (false, false) => Color::from_rgb8(0xee, 0xf5, 0xff),
            (false, true) => Color::from_rgb8(0xf3, 0xf0, 0xff),
            (true, false) => Color::from_rgb8(0x1a, 0x26, 0x3a),
            (true, true) => Color::from_rgb8(0x24, 0x1f, 0x3a),
        };
        let end = if dark(theme) {
            Color::from_rgb8(0x12, 0x16, 0x1f)
        } else {
            Color::WHITE
        };
        let mut style = surface(tint, fg(theme));
        style.background = Some(Background::Gradient(Gradient::Linear(
            Linear::new(Radians(2.1)).add_stop(0.0, tint).add_stop(1.0, end),
        )));
        style.border = Border {
            color: hair(theme),
            width: 1.0,
            radius: 12.0.into(),
        };
        style
    })
}

pub fn screenshot_class() -> theme::Container<'static> {
    theme::Container::custom(|theme| {
        let mut style = surface(
            if dark(theme) {
                Color::from_rgb8(0x1a, 0x1f, 0x29)
            } else {
                Color::from_rgb8(0xfb, 0xfb, 0xfd)
            },
            fg(theme),
        );
        style.border = Border {
            color: hair(theme),
            width: 1.0,
            radius: 9.0.into(),
        };
        style
    })
}

pub fn badge_class(on_accent: bool) -> theme::Container<'static> {
    theme::Container::custom(move |theme| {
        let (bg, color) = if on_accent {
            (Color::from_rgba(1.0, 1.0, 1.0, 0.28), Color::WHITE)
        } else {
            (wash(theme, 0.08), secondary(theme))
        };
        let mut style = surface(bg, color);
        style.border.radius = 8.0.into();
        style
    })
}

pub fn ask_tile(_theme: &Theme) -> container::Style {
    let mut style = surface(ACCENT, Color::WHITE);
    style.border.radius = 8.0.into();
    style
}

fn custom(f: fn(&Theme, bool) -> button::Style) -> theme::Button {
    theme::Button::Custom {
        active: Box::new(move |_, theme| f(theme, false)),
        hovered: Box::new(move |_, theme| f(theme, true)),
        pressed: Box::new(move |_, theme| f(theme, true)),
        disabled: Box::new(move |theme| {
            let mut style = f(theme, false);
            style.text_color = Some(tertiary(theme));
            style.icon_color = Some(tertiary(theme));
            style
        }),
    }
}

pub fn sidebar_item(active: bool) -> theme::Button {
    theme::Button::Custom {
        active: Box::new(move |_, theme| sidebar_item_style(theme, active, false)),
        hovered: Box::new(move |_, theme| sidebar_item_style(theme, active, true)),
        pressed: Box::new(move |_, theme| sidebar_item_style(theme, active, true)),
        disabled: Box::new(move |theme| sidebar_item_style(theme, active, false)),
    }
}

fn sidebar_item_style(theme: &Theme, active: bool, hovered: bool) -> button::Style {
    let mut style = button::Style::new();
    style.border_radius = 6.0.into();
    if active {
        style.background = Some(Background::Color(ACCENT));
        style.text_color = Some(Color::WHITE);
        style.icon_color = Some(Color::WHITE);
    } else {
        if hovered {
            style.background = Some(Background::Color(wash(theme, 0.045)));
        }
        style.text_color = Some(fg(theme));
        style.icon_color = Some(Color::from_rgb8(0x1f, 0x7a, 0xe0));
    }
    style
}

/// Plain toolbar/text button: no fill until hovered.
pub fn tool() -> theme::Button {
    custom(|theme, hovered| {
        let mut style = button::Style::new();
        style.border_radius = 6.0.into();
        if hovered {
            style.background = Some(Background::Color(wash(theme, 0.05)));
        }
        let color = if dark(theme) {
            Color::from_rgb8(0xc7, 0xd0, 0xde)
        } else {
            Color::from_rgb8(0x4a, 0x4a, 0x4f)
        };
        style.text_color = Some(color);
        style.icon_color = Some(color);
        style
    })
}

pub fn close_tool() -> theme::Button {
    custom(|theme, hovered| {
        let mut style = button::Style::new();
        style.border_radius = 6.0.into();
        if hovered {
            style.background = Some(Background::Color(DANGER));
            style.text_color = Some(Color::WHITE);
            style.icon_color = Some(Color::WHITE);
        } else {
            let color = secondary(theme);
            style.text_color = Some(color);
            style.icon_color = Some(color);
        }
        style
    })
}

pub fn link() -> theme::Button {
    custom(|_, hovered| {
        let mut style = button::Style::new();
        let color = if hovered {
            Color::from_rgb8(0x00, 0x71, 0xe3)
        } else {
            ACCENT
        };
        style.text_color = Some(color);
        style.icon_color = Some(color);
        style
    })
}

pub fn menu_item() -> theme::Button {
    custom(|theme, hovered| {
        let mut style = button::Style::new();
        style.border_radius = 5.0.into();
        if hovered {
            style.background = Some(Background::Color(ACCENT));
            style.text_color = Some(Color::WHITE);
            style.icon_color = Some(Color::WHITE);
        } else {
            style.text_color = Some(fg(theme));
            style.icon_color = Some(fg(theme));
        }
        style
    })
}

#[derive(Clone, Copy)]
pub enum Pill {
    Accent,
    Solid,
    Plain,
    Danger,
}

pub fn pill(kind: Pill) -> theme::Button {
    theme::Button::Custom {
        active: Box::new(move |_, theme| pill_style(theme, kind, false)),
        hovered: Box::new(move |_, theme| pill_style(theme, kind, true)),
        pressed: Box::new(move |_, theme| pill_style(theme, kind, true)),
        disabled: Box::new(move |theme| {
            let mut style = pill_style(theme, kind, false);
            style.text_color = Some(tertiary(theme));
            style
        }),
    }
}

fn pill_style(theme: &Theme, kind: Pill, hovered: bool) -> button::Style {
    let mut style = button::Style::new();
    style.border_radius = 12.0.into();
    let lift = if hovered { 0.05 } else { 0.0 };
    let (bg, color) = match kind {
        Pill::Accent => (Color { a: 0.10 + lift, ..ACCENT }, ACCENT),
        Pill::Solid => (
            if hovered {
                Color::from_rgb8(0x00, 0x71, 0xe3)
            } else {
                ACCENT
            },
            Color::WHITE,
        ),
        Pill::Plain => (wash(theme, 0.055 + lift), fg(theme)),
        Pill::Danger => (Color { a: 0.10 + lift, ..DANGER }, DANGER),
    };
    style.background = Some(Background::Color(bg));
    style.text_color = Some(color);
    style.icon_color = Some(color);
    style
}

pub fn pill_button<'a, M: Clone + 'a>(
    label: impl Into<String>,
    kind: Pill,
    on_press: Option<M>,
) -> cosmic::Element<'a, M> {
    let text = cosmic::widget::text::body(label.into()).font(cosmic::font::semibold());
    cosmic::widget::button::custom(text)
        .padding([3, 14])
        .class(pill(kind))
        .on_press_maybe(on_press)
        .into()
}
