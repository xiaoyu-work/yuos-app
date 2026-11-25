// SPDX-License-Identifier: GPL-3.0-only

use cosmic::iced::{Background, Border, Color, Shadow, Vector};
use cosmic::widget::{button, container};
use cosmic::{Theme, theme};

const CHROME_ALPHA: f32 = 0.62;
const CONTENT_ALPHA: f32 = 0.92;
const DASHBOARD_ALPHA: f32 = 0.72;
const PATH_ALPHA: f32 = 0.58;

fn material(color: impl Into<Color>, alpha: f32, frosted: bool) -> Color {
    let mut color = color.into();
    if frosted {
        color.a = alpha;
    }
    color
}

fn accent(theme: &Theme, alpha: f32) -> Color {
    let mut color: Color = theme.cosmic().accent_color().into();
    color.a = alpha;
    color
}

/// Neutral hairline for glass chrome.
///
/// `on_bg_color` is near-black on a light theme and near-white on a dark one,
/// so one low-alpha expression edges a surface correctly in both without
/// tinting the glass it borders.
fn hairline(theme: &Theme, alpha: f32) -> Color {
    let mut color: Color = theme.cosmic().on_bg_color().into();
    color.a = alpha;
    color
}

fn depth_shadow(theme: &Theme, alpha: f32, y: f32, blur_radius: f32) -> Shadow {
    let alpha = if theme.cosmic().is_dark {
        alpha
    } else {
        alpha * 0.45
    };
    Shadow {
        color: Color::from_rgba(0.0, 0.0, 0.0, alpha),
        offset: Vector::new(0.0, y),
        blur_radius,
    }
}

pub const ACCENT: Color = Color::from_rgb(10.0 / 255.0, 132.0 / 255.0, 1.0);
pub const SIDEBAR_ICON: Color = Color::from_rgb(31.0 / 255.0, 122.0 / 255.0, 224.0 / 255.0);
pub const FOLDER_ICON: Color = SIDEBAR_ICON;
pub const FILE_ICON: Color = Color::from_rgb(142.0 / 255.0, 142.0 / 255.0, 147.0 / 255.0);
pub const ROW_ALT: Color = Color::from_rgb(246.0 / 255.0, 247.0 / 255.0, 249.0 / 255.0);
pub const BODY_SIZE: f32 = 13.0;
pub const SMALL_SIZE: f32 = 11.5;

pub fn text_primary(theme: &Theme) -> Color {
    if theme.cosmic().is_dark {
        Color::from_rgb8(0xee, 0xf2, 0xf7)
    } else {
        Color::from_rgb8(0x1d, 0x1d, 0x1f)
    }
}

pub fn text_secondary(theme: &Theme) -> Color {
    if theme.cosmic().is_dark {
        Color::from_rgb8(0x9a, 0x9a, 0xa0)
    } else {
        Color::from_rgb8(0x6e, 0x6e, 0x73)
    }
}

pub fn text_tertiary(theme: &Theme) -> Color {
    if theme.cosmic().is_dark {
        Color::from_rgb8(0x7c, 0x7c, 0x82)
    } else {
        Color::from_rgb8(0xa1, 0xa1, 0xa6)
    }
}

fn rule_color(theme: &Theme) -> Color {
    // Opaque equivalent of rgba(0,0,0,.09) on white; translucent rules render black.
    if theme.cosmic().is_dark {
        Color::from_rgb8(0x32, 0x34, 0x38)
    } else {
        Color::from_rgb8(0xe8, 0xe8, 0xe8)
    }
}

fn chrome_bg(theme: &Theme) -> Color {
    if theme.cosmic().is_dark {
        Color::from_rgb8(0x1b, 0x1d, 0x22)
    } else {
        Color::WHITE
    }
}

fn plain_surface(bg: Color, fg: Color) -> container::Style {
    container::Style {
        icon_color: Some(fg),
        text_color: Some(fg),
        background: Some(Background::Color(bg)),
        border: Border::default(),
        snap: true,
        ..Default::default()
    }
}

pub fn toolbar(theme: &Theme) -> container::Style {
    top_toolbar_surface(theme)
}

pub fn tab_strip(theme: &Theme) -> container::Style {
    action_strip_surface(theme)
}

pub fn content(theme: &Theme) -> container::Style {
    let is_dark = theme.cosmic().is_dark;
    let bg = if is_dark {
        Color::from_rgb8(0x12, 0x16, 0x1f)
    } else {
        Color::WHITE
    };
    let fg = if is_dark {
        Color::from_rgb8(0xee, 0xf2, 0xf7)
    } else {
        Color::from_rgb8(0x1b, 0x22, 0x30)
    };
    container::Style {
        icon_color: Some(fg),
        text_color: Some(fg),
        background: Some(Background::Color(bg)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        snap: true,
        ..Default::default()
    }
}

pub fn sidebar_surface(theme: &Theme) -> container::Style {
    let bg = if theme.cosmic().is_dark {
        Color::from_rgb8(0x1f, 0x24, 0x2c)
    } else {
        // rgba(226,236,246,.82) composited over the white window.
        Color::from_rgb8(0xe7, 0xef, 0xf8)
    };
    plain_surface(bg, text_primary(theme))
}

pub fn top_toolbar_surface(theme: &Theme) -> container::Style {
    plain_surface(chrome_bg(theme), text_primary(theme))
}

pub fn action_strip_surface(theme: &Theme) -> container::Style {
    plain_surface(chrome_bg(theme), tool_color(theme))
}

pub fn table_header_surface(theme: &Theme) -> container::Style {
    plain_surface(chrome_bg(theme), text_secondary(theme))
}

pub fn bottom_bar_surface(theme: &Theme) -> container::Style {
    plain_surface(chrome_bg(theme), text_secondary(theme))
}

fn tool_color(theme: &Theme) -> Color {
    if theme.cosmic().is_dark {
        Color::from_rgb8(0xc8, 0xc8, 0xcd)
    } else {
        Color::from_rgb8(0x4a, 0x4a, 0x4f)
    }
}
pub fn ai_drawer_surface(theme: &Theme) -> container::Style {
    let is_dark = theme.cosmic().is_dark;
    let bg = if is_dark {
        Color::from_rgb8(0x16, 0x1b, 0x24)
    } else {
        Color::from_rgb8(0xf9, 0xfb, 0xfe)
    };
    let fg = if is_dark {
        Color::from_rgb8(0xee, 0xf2, 0xf7)
    } else {
        Color::from_rgb8(0x1b, 0x22, 0x30)
    };
    container::Style {
        icon_color: Some(fg),
        text_color: Some(fg),
        background: Some(Background::Color(bg)),
        border: Border::default(),
        snap: true,
        ..Default::default()
    }
}

pub fn ai_header_surface(theme: &Theme) -> container::Style {
    let is_dark = theme.cosmic().is_dark;
    let bg = if is_dark {
        Color::from_rgb8(0x1b, 0x21, 0x2c)
    } else {
        Color::from_rgb8(0xf3, 0xf7, 0xfc)
    };
    let fg = if is_dark {
        Color::from_rgb8(0xee, 0xf2, 0xf7)
    } else {
        Color::from_rgb8(0x18, 0x22, 0x32)
    };
    container::Style {
        icon_color: Some(fg),
        text_color: Some(fg),
        background: Some(Background::Color(bg)),
        border: Border::default(),
        snap: true,
        ..Default::default()
    }
}

pub fn ai_context_bar_surface(theme: &Theme) -> container::Style {
    let is_dark = theme.cosmic().is_dark;
    let bg = if is_dark {
        Color::from_rgb8(0x14, 0x19, 0x22)
    } else {
        Color::WHITE
    };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border::default(),
        snap: true,
        ..Default::default()
    }
}

pub fn ai_pill_surface(theme: &Theme) -> container::Style {
    let is_dark = theme.cosmic().is_dark;
    let (bg, border_col, fg) = if is_dark {
        (
            Color::from_rgba8(0x00, 0x5c, 0xfe, 0.22),
            Color::from_rgba8(0x00, 0x5c, 0xfe, 0.45),
            Color::from_rgb8(0xd8, 0xe7, 0xff),
        )
    } else {
        (
            Color::from_rgb8(0xea, 0xf2, 0xff),
            Color::from_rgb8(0xcf, 0xe0, 0xff),
            Color::from_rgb8(0x27, 0x40, 0x60),
        )
    };
    container::Style {
        icon_color: Some(fg),
        text_color: Some(fg),
        background: Some(Background::Color(bg)),
        border: Border {
            color: border_col,
            width: 1.0,
            radius: 999.0.into(),
        },
        snap: true,
        ..Default::default()
    }
}

pub fn ai_user_bubble(_theme: &Theme) -> container::Style {
    container::Style {
        icon_color: Some(Color::WHITE),
        text_color: Some(Color::WHITE),
        background: Some(Background::Color(Color::from_rgb8(0x00, 0x5c, 0xfe))),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 10.0.into(),
        },
        snap: true,
        ..Default::default()
    }
}

pub fn ai_agent_card(theme: &Theme) -> container::Style {
    let is_dark = theme.cosmic().is_dark;
    let (bg, border_col, fg) = if is_dark {
        (
            Color::from_rgb8(0x1e, 0x25, 0x32),
            Color::from_rgb8(0x2d, 0x36, 0x46),
            Color::from_rgb8(0xee, 0xf2, 0xf7),
        )
    } else {
        (
            Color::WHITE,
            Color::from_rgb8(0xe3, 0xe8, 0xf0),
            Color::from_rgb8(0x1e, 0x29, 0x3b),
        )
    };
    container::Style {
        icon_color: Some(fg),
        text_color: Some(fg),
        background: Some(Background::Color(bg)),
        border: Border {
            color: border_col,
            width: 1.0,
            radius: 10.0.into(),
        },
        snap: true,
        ..Default::default()
    }
}

pub fn view_switcher_group(theme: &Theme) -> container::Style {
    let bg = if theme.cosmic().is_dark {
        Color::from_rgba(1.0, 1.0, 1.0, 0.06)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.045)
    };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 7.0.into(),
        },
        snap: true,
        ..Default::default()
    }
}

pub fn search_input() -> theme::TextInput {
    fn appearance(theme: &Theme, focused: bool) -> cosmic::widget::text_input::Appearance {
        let is_dark = theme.cosmic().is_dark;
        let bg = if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.07)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.05)
        };
        cosmic::widget::text_input::Appearance {
            background: Background::Color(bg),
            border_radius: 8.0.into(),
            border_offset: None,
            border_width: if focused { 1.0 } else { 0.0 },
            border_color: if focused {
                Color { a: 0.55, ..ACCENT }
            } else {
                Color::TRANSPARENT
            },
            label_color: text_secondary(theme),
            placeholder_color: text_tertiary(theme),
            selected_text_color: Color::WHITE,
            icon_color: Some(text_tertiary(theme)),
            text_color: Some(text_primary(theme)),
            selected_fill: ACCENT,
        }
    }
    theme::TextInput::Custom {
        active: Box::new(|theme| appearance(theme, false)),
        error: Box::new(|theme| appearance(theme, true)),
        hovered: Box::new(|theme| appearance(theme, false)),
        focused: Box::new(|theme| appearance(theme, true)),
        disabled: Box::new(|theme| appearance(theme, false)),
    }
}
pub fn divider_rule() -> theme::Rule {
    theme::Rule::Custom(Box::new(|theme| cosmic::iced::widget::rule::Style {
        color: rule_color(theme),
        radius: 0.0.into(),
        fill_mode: cosmic::iced::widget::rule::FillMode::Full,
        snap: true,
    }))
}

pub fn sidebar_rule() -> theme::Rule {
    divider_rule()
}

pub fn sidebar_item_button(active: bool) -> theme::Button {
    theme::Button::Custom {
        active: Box::new(move |_focused, theme| sidebar_item_style(theme, active, false)),
        hovered: Box::new(move |_focused, theme| sidebar_item_style(theme, active, true)),
        pressed: Box::new(move |_focused, theme| sidebar_item_style(theme, active, true)),
        disabled: Box::new(move |theme| sidebar_item_style(theme, active, false)),
    }
}

fn sidebar_item_style(theme: &Theme, active: bool, hovered: bool) -> button::Style {
    let is_dark = theme.cosmic().is_dark;
    let mut style = button::Style::new();
    style.border_radius = 6.0.into();
    if active {
        style.background = Some(Background::Color(ACCENT));
        style.text_color = Some(Color::WHITE);
        style.icon_color = Some(Color::WHITE);
    } else {
        if hovered {
            style.background = Some(Background::Color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.06)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.045)
            }));
        }
        style.text_color = Some(text_primary(theme));
        style.icon_color = Some(SIDEBAR_ICON);
    }
    style
}

pub fn window_light_button(color: Color) -> theme::Button {
    let style = move |hovered: bool| {
        let mut s = button::Style::new();
        let mut c = color;
        if hovered {
            c.r *= 0.88;
            c.g *= 0.88;
            c.b *= 0.88;
        }
        s.background = Some(Background::Color(c));
        s.border_radius = 6.0.into();
        s.border_width = 0.5;
        s.border_color = Color::from_rgba(0.0, 0.0, 0.0, 0.14);
        s
    };
    theme::Button::Custom {
        active: Box::new(move |_focused, _theme| style(false)),
        hovered: Box::new(move |_focused, _theme| style(true)),
        pressed: Box::new(move |_focused, _theme| style(true)),
        disabled: Box::new(move |_theme| style(false)),
    }
}

pub fn toolbar_icon_button(active: bool) -> theme::Button {
    theme::Button::Custom {
        active: Box::new(move |_focused, theme| toolbar_icon_style(theme, active, false)),
        hovered: Box::new(move |_focused, theme| toolbar_icon_style(theme, active, true)),
        pressed: Box::new(move |_focused, theme| toolbar_icon_style(theme, active, true)),
        disabled: Box::new(move |theme| {
            let mut s = toolbar_icon_style(theme, false, false);
            let dim = if theme.cosmic().is_dark {
                Color::from_rgb8(0x5a, 0x5a, 0x60)
            } else {
                Color::from_rgb8(0xc3, 0xc3, 0xc7)
            };
            s.icon_color = Some(dim);
            s.text_color = Some(dim);
            s
        }),
    }
}

fn toolbar_icon_style(theme: &Theme, active: bool, hovered: bool) -> button::Style {
    let is_dark = theme.cosmic().is_dark;
    let mut style = button::Style::new();
    style.border_radius = 6.0.into();
    if active {
        style.background = Some(Background::Color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.14)
        } else {
            Color::WHITE
        }));
        style.border_width = 0.5;
        style.border_color = Color::from_rgba(0.0, 0.0, 0.0, 0.10);
        style.shadow_offset = Vector::new(0.0, 1.0);
    } else if hovered {
        style.background = Some(Background::Color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.07)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.05)
        }));
    }
    let fg = tool_color(theme);
    style.icon_color = Some(fg);
    style.text_color = Some(fg);
    style
}

pub fn menu_item_button() -> theme::Button {
    let style = |theme: &Theme, hovered: bool| {
        let mut s = button::Style::new();
        s.border_radius = 5.0.into();
        if hovered {
            s.background = Some(Background::Color(ACCENT));
            s.text_color = Some(Color::WHITE);
            s.icon_color = Some(Color::WHITE);
        } else {
            s.text_color = Some(text_primary(theme));
            s.icon_color = Some(text_secondary(theme));
        }
        s
    };
    theme::Button::Custom {
        active: Box::new(move |focused, theme| style(theme, focused)),
        hovered: Box::new(move |_focused, theme| style(theme, true)),
        pressed: Box::new(move |_focused, theme| style(theme, true)),
        disabled: Box::new(move |theme| style(theme, false)),
    }
}
pub fn ai_chip_button() -> theme::Button {
    theme::Button::Custom {
        active: Box::new(|_focused, theme| ai_chip_style(theme, false)),
        hovered: Box::new(|_focused, theme| ai_chip_style(theme, true)),
        pressed: Box::new(|_focused, theme| ai_chip_style(theme, true)),
        disabled: Box::new(|theme| ai_chip_style(theme, false)),
    }
}

fn ai_chip_style(theme: &Theme, hovered: bool) -> button::Style {
    let is_dark = theme.cosmic().is_dark;
    let mut style = button::Style::new();
    style.border_radius = 6.0.into();
    style.border_width = 1.0;
    if hovered {
        style.background = Some(Background::Color(if is_dark {
            Color::from_rgba8(0x00, 0x5c, 0xfe, 0.18)
        } else {
            Color::from_rgb8(0xf2, 0xf7, 0xff)
        }));
        style.border_color = Color::from_rgb8(0x00, 0x5c, 0xfe);
        style.text_color = Some(Color::from_rgb8(0x00, 0x5c, 0xfe));
    } else {
        style.background = Some(Background::Color(if is_dark {
            Color::from_rgb8(0x1e, 0x25, 0x32)
        } else {
            Color::WHITE
        }));
        style.border_color = if is_dark {
            Color::from_rgb8(0x2d, 0x36, 0x46)
        } else {
            Color::from_rgb8(0xd7, 0xe0, 0xec)
        };
        style.text_color = Some(if is_dark {
            Color::from_rgb8(0xd0, 0xd8, 0xe6)
        } else {
            Color::from_rgb8(0x33, 0x41, 0x55)
        });
    }
    style
}

pub fn ai_send_button() -> theme::Button {
    theme::Button::Custom {
        active: Box::new(|_focused, _theme| {
            let mut s = button::Style::new();
            s.background = Some(Background::Color(Color::from_rgb8(0x00, 0x5c, 0xfe)));
            s.icon_color = Some(Color::WHITE);
            s.text_color = Some(Color::WHITE);
            s.border_radius = 7.0.into();
            s
        }),
        hovered: Box::new(|_focused, _theme| {
            let mut s = button::Style::new();
            s.background = Some(Background::Color(Color::from_rgb8(0x00, 0x4e, 0xd8)));
            s.icon_color = Some(Color::WHITE);
            s.text_color = Some(Color::WHITE);
            s.border_radius = 7.0.into();
            s
        }),
        pressed: Box::new(|_focused, _theme| {
            let mut s = button::Style::new();
            s.background = Some(Background::Color(Color::from_rgb8(0x00, 0x44, 0xbd)));
            s.icon_color = Some(Color::WHITE);
            s.text_color = Some(Color::WHITE);
            s.border_radius = 7.0.into();
            s
        }),
        disabled: Box::new(|_theme| {
            let mut s = button::Style::new();
            s.background = Some(Background::Color(Color::from_rgba8(0x00, 0x5c, 0xfe, 0.45)));
            s.icon_color = Some(Color::WHITE);
            s.text_color = Some(Color::WHITE);
            s.border_radius = 7.0.into();
            s
        }),
    }
}

/// Backdrop for the desktop icon grid.
///
/// In `Mode::Desktop` the tab is stretched across the entire output behind the
/// icon grid, so giving it [`content`]'s surface paints a full-screen sheet
/// over the wallpaper — at `CONTENT_ALPHA` that reads as a large rounded
/// rectangle covering the desktop, hairline and all. Out here the wallpaper is
/// the surface. Keep the foreground colours so icon labels stay legible and
/// drop the fill, border and radius.
pub fn desktop_content(theme: &Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        icon_color: Some(cosmic.on_bg_color().into()),
        text_color: Some(cosmic.on_bg_color().into()),
        background: None,
        ..Default::default()
    }
}

pub fn navigation(theme: &Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        icon_color: Some(cosmic.on_bg_color().into()),
        text_color: Some(cosmic.on_bg_color().into()),
        background: Some(Background::Color(material(
            cosmic.primary.base,
            CHROME_ALPHA,
            cosmic.is_frosted,
        ))),
        border: Border {
            color: hairline(theme, 0.11),
            width: 1.0,
            radius: cosmic.radius_s().into(),
        },
        shadow: depth_shadow(theme, 0.12, 3.0, 18.0),
        snap: true,
    }
}

pub fn path_bar(theme: &Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        icon_color: Some(cosmic.on_bg_component_color().into()),
        text_color: Some(cosmic.on_bg_component_color().into()),
        background: Some(Background::Color(material(
            cosmic.bg_component_color(),
            PATH_ALPHA,
            cosmic.is_frosted,
        ))),
        border: Border {
            color: hairline(theme, 0.12),
            width: 1.0,
            radius: cosmic.radius_l().into(),
        },
        shadow: depth_shadow(theme, 0.10, 3.0, 14.0),
        snap: true,
    }
}

pub fn menu(theme: &Theme) -> container::Style {
    let cosmic = theme.cosmic();
    let base = if cosmic.is_dark {
        Color::from_rgb8(0x26, 0x28, 0x2e)
    } else {
        Color::from_rgb8(0xf6, 0xf6, 0xf8)
    };
    container::Style {
        icon_color: Some(text_secondary(theme)),
        text_color: Some(text_primary(theme)),
        background: Some(Background::Color(material(base, 0.86, cosmic.is_frosted))),
        border: Border {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.18),
            width: 0.5,
            radius: 10.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.20),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 34.0,
        },
        snap: true,
    }
}
pub fn dashboard_card(theme: &Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        icon_color: Some(cosmic.on_bg_color().into()),
        text_color: Some(cosmic.on_bg_color().into()),
        background: Some(Background::Color(material(
            cosmic.bg_component_color(),
            DASHBOARD_ALPHA,
            cosmic.is_frosted,
        ))),
        border: Border {
            color: hairline(theme, 0.09),
            width: 1.0,
            radius: cosmic.radius_l().into(),
        },
        shadow: depth_shadow(theme, 0.10, 4.0, 18.0),
        snap: true,
    }
}

pub fn dashboard_accent_card(theme: &Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        icon_color: Some(cosmic.accent_color().into()),
        text_color: Some(cosmic.on_bg_color().into()),
        background: Some(Background::Color(accent(
            theme,
            if cosmic.is_frosted { 0.10 } else { 0.07 },
        ))),
        border: Border {
            color: accent(theme, 0.22),
            width: 1.0,
            radius: cosmic.radius_l().into(),
        },
        shadow: depth_shadow(theme, 0.08, 3.0, 16.0),
        snap: true,
    }
}

pub fn dashboard_button() -> theme::Button {
    theme::Button::Custom {
        active: Box::new(|focused, theme| dashboard_button_style(theme, focused, 0.44, 0.12, 0.06)),
        hovered: Box::new(|focused, theme| {
            dashboard_button_style(theme, focused, 0.62, 0.24, 0.12)
        }),
        pressed: Box::new(|focused, theme| {
            dashboard_button_style(theme, focused, 0.72, 0.30, 0.06)
        }),
        disabled: Box::new(|theme| dashboard_button_style(theme, false, 0.30, 0.08, 0.0)),
    }
}

fn dashboard_button_style(
    theme: &Theme,
    focused: bool,
    background_alpha: f32,
    border_alpha: f32,
    shadow_alpha: f32,
) -> button::Style {
    let cosmic = theme.cosmic();
    let mut style = button::Style::new();
    style.background = Some(Background::Color(material(
        cosmic.bg_component_color(),
        background_alpha,
        cosmic.is_frosted,
    )));
    style.icon_color = Some(cosmic.accent_color().into());
    style.text_color = Some(cosmic.on_bg_color().into());
    style.border_radius = cosmic.radius_l().into();
    style.border_width = 1.0;
    style.border_color = hairline(theme, border_alpha);
    style.shadow_offset = Vector::new(0.0, if shadow_alpha > 0.0 { 2.0 } else { 0.0 });
    if focused {
        style.outline_width = 1.0;
        style.outline_color = cosmic.accent_color().into();
    }
    style
}

pub fn ask_yu_button() -> theme::Button {
    theme::Button::Custom {
        active: Box::new(|focused, theme| ask_yu_style(theme, focused, 0.10, 0.22)),
        hovered: Box::new(|focused, theme| ask_yu_style(theme, focused, 0.18, 0.34)),
        pressed: Box::new(|focused, theme| ask_yu_style(theme, focused, 0.24, 0.44)),
        disabled: Box::new(|theme| ask_yu_style(theme, false, 0.06, 0.10)),
    }
}

fn ask_yu_style(
    theme: &Theme,
    focused: bool,
    background_alpha: f32,
    border_alpha: f32,
) -> button::Style {
    let cosmic = theme.cosmic();
    let mut style = button::Style::new();
    style.background = Some(Background::Color(accent(theme, background_alpha)));
    style.icon_color = Some(cosmic.accent_color().into());
    style.text_color = Some(cosmic.accent_text_color().into());
    style.border_radius = cosmic.radius_m().into();
    style.border_width = 1.0;
    style.border_color = accent(theme, border_alpha);
    style.shadow_offset = Vector::new(0.0, if background_alpha >= 0.18 { 1.0 } else { 0.0 });
    if focused {
        style.outline_width = 1.0;
        style.outline_color = cosmic.accent_color().into();
    }
    style
}
