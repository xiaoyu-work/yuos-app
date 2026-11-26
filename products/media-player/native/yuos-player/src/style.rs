// Copyright 2024 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

use cosmic::iced::widget::slider;
use cosmic::iced::{Border, Color, Shadow, Vector};
use cosmic::theme::{Button, Container, Theme};
use cosmic::widget::{button, container};
use std::rc::Rc;

fn alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

fn accent(theme: &Theme) -> Color {
    if theme.cosmic().is_dark {
        Color::from_rgb8(0x64, 0xa9, 0xff)
    } else {
        Color::from_rgb8(0x00, 0x68, 0xd4)
    }
}

pub fn surface_color(theme: &Theme) -> Color {
    if theme.cosmic().is_dark {
        theme.cosmic().bg_color().into()
    } else {
        Color::WHITE
    }
}

pub fn surface() -> Container<'static> {
    Container::custom(|theme| container::Style::default().background(surface_color(theme)))
}

pub fn video_stage() -> Container<'static> {
    Container::custom(|_| container::Style::default().background(Color::BLACK))
}

pub fn sidebar() -> Container<'static> {
    Container::custom(|theme| {
        let cosmic = theme.cosmic();
        let color = if cosmic.is_dark {
            cosmic.primary.base.into()
        } else {
            Color::from_rgb8(231, 238, 248)
        };
        container::Style::default().background(color)
    })
}

pub fn divider() -> Container<'static> {
    Container::custom(|theme| {
        container::Style::default().background(alpha(theme.cosmic().on_bg_color().into(), 0.12))
    })
}

pub fn muted() -> Container<'static> {
    Container::custom(|theme| {
        let color = alpha(theme.cosmic().on_bg_color().into(), 0.72);
        container::Style {
            text_color: Some(color),
            icon_color: Some(color),
            ..Default::default()
        }
    })
}

pub fn artwork() -> Container<'static> {
    Container::custom(|theme| container::Style {
        border: Border {
            width: 1.0,
            color: alpha(theme.cosmic().on_bg_color().into(), 0.12),
            radius: 8.0.into(),
        },
        icon_color: Some(accent(theme)),
        ..Default::default()
    })
}

pub fn popover() -> Container<'static> {
    Container::custom(|theme| container::Style {
        background: Some(surface_color(theme).into()),
        border: Border {
            width: 1.0,
            color: alpha(theme.cosmic().on_bg_color().into(), 0.16),
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: alpha(Color::BLACK, 0.10),
            offset: Vector::new(0.0, 3.0),
            blur_radius: 10.0,
        },
        ..Default::default()
    })
}

pub fn control(selected: bool) -> Button {
    chrome_button(selected, false, 6.0)
}

pub fn outline_button() -> Button {
    chrome_button(false, true, 6.0)
}

pub fn play_button() -> Button {
    chrome_button(true, false, 18.0)
}

fn chrome_button(selected: bool, outline: bool, radius: f32) -> Button {
    let state = move |focused, theme: &Theme, interaction| {
        let cosmic = theme.cosmic();
        let fg: Color = cosmic.on_bg_color().into();
        let (color, background) = if selected {
            (
                Color::WHITE,
                Some(
                    match interaction {
                        1 => Color::from_rgb8(0x00, 0x5d, 0xc1),
                        2 => Color::from_rgb8(0x00, 0x51, 0xab),
                        _ => Color::from_rgb8(0x00, 0x68, 0xd4),
                    }
                    .into(),
                ),
            )
        } else {
            (
                fg,
                (interaction > 0)
                    .then(|| alpha(fg, if interaction == 1 { 0.07 } else { 0.13 }).into()),
            )
        };
        let mut style = button::Style::new();
        style.background = background;
        style.text_color = Some(color);
        style.icon_color = Some(color);
        style.border_radius = radius.into();
        if focused || outline {
            style.border_width = if focused { 2.0 } else { 1.0 };
            style.border_color = if focused {
                accent(theme)
            } else {
                alpha(fg, 0.18)
            };
        }
        style
    };
    Button::Custom {
        active: Box::new(move |focused, theme| state(focused, theme, 0)),
        hovered: Box::new(move |focused, theme| state(focused, theme, 1)),
        pressed: Box::new(move |focused, theme| state(focused, theme, 2)),
        disabled: Box::new(move |theme| state(false, theme, 0)),
    }
}

pub fn scrubber() -> cosmic::theme::iced::Slider {
    cosmic::theme::iced::Slider::Custom {
        active: Rc::new(|theme| scrubber_style(theme, false)),
        hovered: Rc::new(|theme| scrubber_style(theme, true)),
        dragging: Rc::new(|theme| scrubber_style(theme, true)),
    }
}

fn scrubber_style(theme: &Theme, highlighted: bool) -> slider::Style {
    let cosmic = theme.cosmic();
    let accent = accent(theme);
    let edge = if highlighted {
        accent
    } else {
        alpha(cosmic.on_bg_color().into(), 0.25)
    };
    slider::Style {
        rail: slider::Rail {
            backgrounds: (
                accent.into(),
                alpha(cosmic.on_bg_color().into(), 0.15).into(),
            ),
            width: 4.0,
            border: Border {
                radius: 2.0.into(),
                ..Default::default()
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 6.0 },
            background: cosmic.bg_color().into(),
            border_width: 1.0,
            border_color: edge,
        },
        breakpoint: slider::Breakpoint { color: accent },
    }
}

pub fn accent_text() -> Container<'static> {
    Container::custom(|theme| container::Style {
        text_color: Some(accent(theme)),
        ..Default::default()
    })
}

pub fn loop_range() -> Container<'static> {
    Container::custom(|theme| container::Style::default().background(alpha(accent(theme), 0.22)))
}

pub fn loop_marker() -> Container<'static> {
    Container::custom(|theme| container::Style::default().background(accent(theme)))
}
