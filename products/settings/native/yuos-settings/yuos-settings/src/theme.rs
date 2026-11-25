// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

use cosmic::{
    cosmic_theme::palette::WithAlpha,
    iced::{Background, Border, Color, Shadow},
    theme,
};

fn page_fill(cosmic: &cosmic::cosmic_theme::Theme) -> Color {
    if cosmic.is_high_contrast {
        cosmic.background.base.into()
    } else if cosmic.is_dark {
        Color::from_rgb8(28, 28, 30)
    } else {
        Color::WHITE
    }
}

#[must_use]
pub fn icon_tile() -> cosmic::theme::Container<'static> {
    theme::Container::custom(|theme| {
        let cosmic = theme.cosmic();

        cosmic::widget::container::Style {
            icon_color: Some(cosmic.accent.on.into()),
            text_color: Some(cosmic.accent.on.into()),
            background: Some(Background::Color(cosmic.accent.base.into())),
            border: Border {
                color: Color::TRANSPARENT,
                radius: 7.0.into(),
                width: 0.0,
            },
            shadow: Shadow::default(),
            snap: false,
        }
    })
}

#[must_use]
pub fn sidebar_icon_tile(id: &str) -> cosmic::theme::Container<'static> {
    let color = match id {
        "desktop" => Color::from_rgb8(94, 92, 230),
        "sound" => Color::from_rgb8(255, 55, 95),
        "power" => Color::from_rgb8(48, 185, 77),
        "input-devices" | "system" => Color::from_rgb8(142, 142, 147),
        "agent" => Color::from_rgb8(138, 108, 247),
        "time" => Color::from_rgb8(255, 159, 10),
        _ => Color::from_rgb8(10, 132, 255),
    };
    theme::Container::custom(move |theme| {
        let cosmic = theme.cosmic();
        let (background, foreground) = if cosmic.is_high_contrast {
            (cosmic.accent.base.into(), cosmic.accent.on.into())
        } else {
            (color, Color::WHITE)
        };
        cosmic::widget::container::Style {
            icon_color: Some(foreground),
            text_color: Some(foreground),
            background: Some(Background::Color(background)),
            border: Border {
                radius: 5.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
}

#[must_use]
pub fn section_card() -> cosmic::theme::Container<'static> {
    theme::Container::custom(|theme| {
        let cosmic = theme.cosmic();

        cosmic::widget::container::Style {
            icon_color: None,
            text_color: None,
            background: Some(Background::Color(page_fill(cosmic))),
            border: Border {
                color: Color::from(cosmic.on_bg_color().with_alpha(if cosmic.is_high_contrast {
                    0.5
                } else {
                    0.10
                })),
                // List buttons use this same radius for their hover backgrounds.
                radius: cosmic.corner_radii.radius_s.into(),
                width: 0.5,
            },
            shadow: Shadow::default(),
            snap: false,
        }
    })
}

#[must_use]
pub fn segment_track() -> cosmic::theme::Container<'static> {
    theme::Container::custom(|theme| {
        let cosmic = theme.cosmic();
        cosmic::widget::container::Style {
            background: Some(Background::Color(if cosmic.is_high_contrast {
                cosmic.bg_component_color().into()
            } else if cosmic.is_dark {
                Color::from_rgb8(44, 44, 46)
            } else {
                Color::from_rgb8(242, 242, 247)
            })),
            border: Border {
                radius: cosmic.corner_radii.radius_s.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
}

#[must_use]
pub fn page() -> cosmic::theme::Container<'static> {
    theme::Container::custom(|theme| {
        let cosmic = theme.cosmic();
        cosmic::widget::container::Style {
            text_color: Some(cosmic.on_bg_color().into()),
            icon_color: Some(cosmic.on_bg_color().into()),
            background: Some(Background::Color(page_fill(cosmic))),
            ..Default::default()
        }
    })
}

#[must_use]
pub fn sidebar() -> cosmic::theme::Container<'static> {
    theme::Container::custom(|theme| {
        let cosmic = theme.cosmic();
        cosmic::widget::container::Style {
            text_color: Some(cosmic.on_bg_color().into()),
            icon_color: Some(cosmic.on_bg_color().into()),
            background: Some(Background::Color(if cosmic.is_high_contrast {
                cosmic.primary.base.into()
            } else if cosmic.is_dark {
                Color::from_rgb8(37, 42, 49)
            } else {
                Color::from_rgb8(226, 236, 246)
            })),
            border: Border {
                color: cosmic.on_bg_color().with_alpha(0.09).into(),
                width: 0.5,
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    })
}

#[must_use]
pub fn navigation_button(selected: bool) -> cosmic::theme::Button {
    let style = move |focused: bool, theme: &cosmic::Theme, hovered: bool| {
        let cosmic = theme.cosmic();
        let foreground = if selected {
            cosmic.accent.on
        } else {
            cosmic.on_bg_color()
        };
        cosmic::widget::button::Style {
            background: Some(Background::Color(if selected {
                cosmic.accent.base.into()
            } else if hovered {
                cosmic.on_bg_color().with_alpha(0.05).into()
            } else {
                Color::TRANSPARENT
            })),
            border_radius: 6.0.into(),
            outline_color: cosmic.accent.base.into(),
            outline_width: if focused { 1.5 } else { 0.0 },
            text_color: Some(foreground.into()),
            icon_color: Some(foreground.into()),
            ..Default::default()
        }
    };
    theme::Button::Custom {
        active: Box::new(move |focused, theme| style(focused, theme, false)),
        hovered: Box::new(move |focused, theme| style(focused, theme, true)),
        pressed: Box::new(move |focused, theme| style(focused, theme, true)),
        disabled: Box::new(move |theme| style(false, theme, false)),
    }
}

#[must_use]
pub fn color_swatch(color: Color, selected: bool) -> cosmic::theme::Button {
    let style = move |focused: bool, theme: &cosmic::Theme| cosmic::widget::button::Style {
        background: Some(Background::Color(color)),
        border_radius: 999.0.into(),
        border_width: 0.5,
        border_color: Color::from_rgba(0.0, 0.0, 0.0, 0.15),
        outline_width: if selected || focused { 1.5 } else { 0.0 },
        outline_color: theme.cosmic().accent.base.into(),
        ..Default::default()
    };
    theme::Button::Custom {
        active: Box::new(style),
        hovered: Box::new(style),
        pressed: Box::new(style),
        disabled: Box::new(move |theme| style(false, theme)),
    }
}

#[must_use]
pub fn segment_button(selected: bool) -> cosmic::theme::Button {
    let style = move |focused: bool, theme: &cosmic::Theme| {
        let cosmic = theme.cosmic();
        cosmic::widget::button::Style {
            background: Some(Background::Color(if selected {
                if cosmic.is_dark {
                    Color::from_rgb8(58, 58, 60)
                } else {
                    Color::WHITE
                }
            } else {
                Color::TRANSPARENT
            })),
            border_radius: 5.0.into(),
            border_width: if selected { 0.5 } else { 0.0 },
            border_color: cosmic.on_bg_color().with_alpha(0.10).into(),
            text_color: Some(cosmic.on_bg_color().into()),
            outline_width: if focused { 1.5 } else { 0.0 },
            outline_color: cosmic.accent.base.into(),
            ..Default::default()
        }
    };
    theme::Button::Custom {
        active: Box::new(style),
        hovered: Box::new(style),
        pressed: Box::new(style),
        disabled: Box::new(move |theme| style(false, theme)),
    }
}

#[must_use]
pub fn display_container_frame() -> cosmic::theme::Container<'static> {
    theme::Container::custom(|theme| {
        let cosmic = theme.cosmic();
        cosmic::widget::container::Style {
            icon_color: None,
            text_color: None,
            background: Some(cosmic::iced::Background::Color(cosmic::iced::Color::WHITE)),
            border: Border {
                color: cosmic::iced::Color::WHITE,
                radius: cosmic.corner_radii.radius_xs.into(),
                width: 3.0,
            },
            shadow: Default::default(),
            snap: true,
        }
    })
}

#[must_use]
pub fn display_container_screen() -> cosmic::theme::Container<'static> {
    theme::Container::custom(|theme| {
        let cosmic = theme.cosmic();
        cosmic::widget::container::Style {
            icon_color: None,
            text_color: None,
            background: Some(cosmic::iced::Background::Color(cosmic::iced::Color::BLACK)),
            border: Border {
                color: cosmic::iced::Color::BLACK,
                radius: cosmic.corner_radii.radius_0.into(),
                width: 0.0,
            },
            shadow: Default::default(),
            snap: true,
        }
    })
}
