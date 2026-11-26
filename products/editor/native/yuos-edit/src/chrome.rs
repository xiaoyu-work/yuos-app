// SPDX-License-Identifier: GPL-3.0-only

use cosmic::{
    Element, Theme,
    iced::{Alignment, Background, Border, Color, Length, widget::text::Wrapping},
    theme,
    widget::{self, button, container, segmented_button},
};

use crate::{Message, icon_cache_get};

pub fn label<'a>(text: impl Into<std::borrow::Cow<'a, str>>) -> widget::Text<'a, Theme> {
    widget::text(text.into()).size(13)
}

pub fn title<'a>(text: impl Into<std::borrow::Cow<'a, str>>, size: f32) -> widget::Text<'a, Theme> {
    widget::text(text.into())
        .font(cosmic::font::semibold())
        .size(size)
}

pub fn muted(theme: &Theme) -> cosmic::iced::advanced::widget::text::Style {
    let mut color = theme.cosmic().background.on;
    if !theme.cosmic().is_high_contrast {
        color.alpha *= 0.72;
    }
    cosmic::iced::advanced::widget::text::Style {
        color: Some(color.into()),
    }
}

pub fn caption<'a>(text: impl Into<std::borrow::Cow<'a, str>>) -> widget::Text<'a, Theme> {
    widget::text(text.into())
        .size(11)
        .class(theme::Text::Custom(muted))
}

fn hairline(theme: &Theme) -> container::Style {
    let cosmic = theme.cosmic();
    let mut color = cosmic.background.on;
    color.alpha *= if cosmic.is_high_contrast { 0.48 } else { 0.14 };
    container::Style {
        background: Some(Background::Color(color.into())),
        ..Default::default()
    }
}

pub fn rule<'a, M: 'a>() -> Element<'a, M> {
    widget::container(widget::space::vertical())
        .width(Length::Fill)
        .height(1)
        .class(theme::Container::custom(hairline))
        .into()
}

pub fn seam<'a, M: 'a>() -> Element<'a, M> {
    widget::container(widget::space::horizontal())
        .width(1)
        .height(Length::Fill)
        .class(theme::Container::custom(hairline))
        .into()
}

fn accent(theme: &Theme) -> Color {
    if theme.cosmic().is_high_contrast {
        theme.cosmic().accent_text_color().into()
    } else if theme.theme_type.is_dark() {
        Color::from_rgb8(124, 189, 255)
    } else {
        Color::from_rgb8(0, 113, 227)
    }
}

pub fn sidebar(theme: &Theme) -> container::Style {
    let cosmic = theme.cosmic();
    let accent = accent(theme);
    let mut base = cosmic.background.base;
    if !cosmic.is_high_contrast {
        let tint = |base, accent| base * 0.91 + accent * 0.09;
        base.red = tint(base.red, accent.r);
        base.green = tint(base.green, accent.g);
        base.blue = tint(base.blue, accent.b);
        base.alpha *= 0.84;
    }
    container::Style {
        background: Some(Background::Color(base.into())),
        ..Default::default()
    }
}

pub fn content(theme: &Theme) -> container::Style {
    container::Style {
        border: Border::default(),
        ..theme::Container::primary(theme.cosmic())
    }
}

pub fn tabs(theme: &Theme) -> segmented_button::Appearance {
    use segmented_button::{ItemAppearance, ItemStatusAppearance, StyleSheet};

    let native = theme.horizontal(&theme::SegmentedButton::TabBar);
    let item = ItemAppearance::default();
    segmented_button::Appearance {
        active_width: 2.0,
        active: ItemStatusAppearance {
            first: item,
            middle: item,
            last: item,
            text_color: accent(theme),
            ..Default::default()
        },
        ..native
    }
}

pub fn project_tree(theme: &Theme) -> segmented_button::Appearance {
    use segmented_button::{ItemAppearance, ItemStatusAppearance, StyleSheet};

    let native = theme.vertical(&theme::SegmentedButton::FileNav);
    let cosmic = theme.cosmic();
    let (background, text_color) = if cosmic.is_high_contrast {
        (cosmic.accent_button.base.into(), cosmic.accent_button.on.into())
    } else if theme.theme_type.is_dark() {
        (accent(theme), Color::BLACK)
    } else {
        (accent(theme), Color::WHITE)
    };
    let item = ItemAppearance {
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
    };
    segmented_button::Appearance {
        active: ItemStatusAppearance {
            background: Some(Background::Color(background)),
            text_color,
            first: item,
            middle: item,
            last: item,
        },
        ..native
    }
}

pub fn tool<'a, M: Clone + 'static>(
    icon: &'static str,
    hint: String,
    message: M,
    active: bool,
) -> Element<'a, M> {
    let button = button::custom(icon_cache_get(icon, 16))
        .on_press(message)
        .padding(6)
        .class(theme::Button::IconVertical)
        .selected(active);
    widget::tooltip(button, label(hint), widget::tooltip::Position::Bottom).into()
}

pub fn field_action<'a>(text: String, message: Message) -> Element<'a, Message> {
    button::custom(label(text).wrapping(Wrapping::None))
        .padding([4, 8])
        .class(theme::Button::Standard)
        .on_press(message)
        .into()
}

pub fn section<'a>(
    heading: String,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    widget::column::with_children(vec![title(heading, 13.0).into(), rule(), content.into()])
        .spacing(6)
        .width(Length::Fill)
        .into()
}

pub fn setting<'a>(name: String, control: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    widget::column::with_children(vec![label(name).into(), control.into()])
        .spacing(4)
        .width(Length::Fill)
        .into()
}

pub fn inspector<'a>(
    icon: &'static str,
    heading: String,
    content: Element<'a, Message>,
    close: Message,
) -> Element<'a, Message> {
    let header = widget::row::with_children(vec![
        icon_cache_get(icon, 16).into(),
        title(heading, 13.0).into(),
        widget::space::horizontal().into(),
        tool(
            "window-close-symbolic",
            crate::fl!("close-panel"),
            close,
            false,
        ),
    ])
    .align_y(Alignment::Center)
    .spacing(8)
    .padding([6, 10]);
    widget::row::with_children(vec![
        seam(),
        widget::column::with_children(vec![
            header.into(),
            rule(),
            widget::scrollable(widget::container(content).width(Length::Fill).padding(12))
                .height(Length::Fill)
                .into(),
        ])
        .width(300)
        .height(Length::Fill)
        .into(),
    ])
    .height(Length::Fill)
    .into()
}
