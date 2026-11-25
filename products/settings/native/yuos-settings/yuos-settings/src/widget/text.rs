// SPDX-License-Identifier: GPL-3.0-only

use std::borrow::Cow;

use cosmic::iced::{Font, core::text::LineHeight, font::Weight};
use cosmic::widget::text::Text;

type SettingsText<'a> = Text<'a, cosmic::Theme, cosmic::Renderer>;

pub fn text<'a>(content: impl Into<Cow<'a, str>> + 'a) -> SettingsText<'a> {
    cosmic::widget::text(content)
        .font(Font::with_name("Noto Sans SC"))
        .size(13)
        .line_height(LineHeight::Absolute(18.0.into()))
}

pub fn body<'a>(content: impl Into<Cow<'a, str>> + 'a) -> SettingsText<'a> {
    text(content)
}

pub fn caption<'a>(content: impl Into<Cow<'a, str>> + 'a) -> SettingsText<'a> {
    text(content)
        .size(11.5)
        .line_height(LineHeight::Absolute(16.0.into()))
}

pub fn heading<'a>(content: impl Into<Cow<'a, str>> + 'a) -> SettingsText<'a> {
    text(content).font(Font {
        weight: Weight::Semibold,
        ..Font::with_name("Noto Sans SC")
    })
}

pub fn caption_heading<'a>(content: impl Into<Cow<'a, str>> + 'a) -> SettingsText<'a> {
    heading(content)
        .size(12)
        .line_height(LineHeight::Absolute(17.0.into()))
}

pub fn title3<'a>(content: impl Into<Cow<'a, str>> + 'a) -> SettingsText<'a> {
    heading(content)
        .size(15)
        .line_height(LineHeight::Absolute(22.0.into()))
}

pub fn title4<'a>(content: impl Into<Cow<'a, str>> + 'a) -> SettingsText<'a> {
    title3(content)
}

pub fn title1<'a>(content: impl Into<Cow<'a, str>> + 'a) -> SettingsText<'a> {
    heading(content)
        .size(29)
        .line_height(LineHeight::Absolute(40.0.into()))
}

pub fn title2<'a>(content: impl Into<Cow<'a, str>> + 'a) -> SettingsText<'a> {
    heading(content)
        .size(24)
        .line_height(LineHeight::Absolute(34.0.into()))
}

pub fn monotext<'a>(content: impl Into<Cow<'a, str>> + 'a) -> SettingsText<'a> {
    cosmic::widget::text::monotext(content)
        .size(12)
        .line_height(LineHeight::Absolute(17.0.into()))
}
