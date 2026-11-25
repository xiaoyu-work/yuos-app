use crate::widget::text;
use cosmic::cosmic_theme::palette::Srgba;
use cosmic::iced::ContentFit;
use cosmic::iced::core::{Alignment, Length};
use cosmic::widget::icon::{from_name, icon};
use cosmic::widget::{self, button, container, list, settings};
use cosmic::{Apply, Element};
use std::collections::HashMap;
use yuos_settings_page::Section;

use super::{ContextView, Message, Page};

#[allow(clippy::too_many_lines)]
pub fn section() -> Section<crate::pages::Message> {
    let (descriptions, label_keys) = i18n();

    Section::default()
        .title(fl!("mode-and-colors"))
        .descriptions(descriptions)
        .view::<Page>(move |_binder, page, section| {
            let label_keys = label_keys.clone();
            let descriptions = &section.descriptions;
            let theme_manager = &page.theme_manager;

            let mut section = crate::widget::yuos_section()
                .title(&section.title)
                .add(theme_mode(page, section, &label_keys))
                .add(auto_switch(page, section, &label_keys))
                .add(accent_color_palette(page, section, &label_keys))
                .add(application_background(page, section, &label_keys))
                .add(container_background(page, section, &label_keys))
                .add(interface_text(page, section, &label_keys))
                .add(control_tint(page, section, &label_keys))
                .add(
                    settings::item::builder(&descriptions[label_keys["window_hint_toggle"]])
                        .toggler(
                            theme_manager.custom_window_hint().is_none(),
                            Message::UseDefaultWindowHint,
                        ),
                );
            if theme_manager.custom_window_hint().is_some() {
                section = section.add(
                    settings::item::builder(&descriptions[label_keys["window_hint"]]).control(
                        page.drawer
                            .accent_window_hint
                            .picker_button(
                                |_| Message::DrawerOpen(ContextView::AccentWindowHint),
                                Some(24),
                            )
                            .width(Length::Fixed(48.0))
                            .height(Length::Fixed(24.0)),
                    ),
                );
            }
            section
                .apply(Element::from)
                .map(crate::pages::Message::Appearance)
        })
}

fn container_background<'a>(
    page: &Page,
    section: &'a Section<crate::pages::Message>,
    labels: &HashMap<String, usize>,
) -> list::ListButton<'a, Message> {
    let descriptions = &section.descriptions;
    let go_next_icon = from_name("go-next-symbolic").handle();

    list::button(
        settings::item::builder(&descriptions[labels["container_bg"]])
            .description(&descriptions[labels["container_bg_desc"]])
            .control(
                if page
                    .drawer
                    .container_background
                    .get_applied_color()
                    .is_some()
                {
                    Element::from(
                        page.drawer
                            .container_background
                            .picker_button(
                                |_| Message::DrawerOpen(ContextView::ContainerBackground),
                                Some(24),
                            )
                            .width(Length::Fixed(48.0))
                            .height(Length::Fixed(24.0)),
                    )
                } else {
                    container(
                        button::text(&descriptions[labels["auto"]])
                            .trailing_icon(go_next_icon.clone())
                            .on_press(Message::DrawerOpen(ContextView::ContainerBackground)),
                    )
                    .into()
                },
            ),
    )
    .on_press(Message::DrawerOpen(ContextView::ContainerBackground))
}

fn application_background<'a>(
    page: &Page,
    section: &'a Section<crate::pages::Message>,
    labels: &HashMap<String, usize>,
) -> list::ListButton<'a, Message> {
    let descriptions = &section.descriptions;

    list::button(
        settings::item::builder(&descriptions[labels["app_bg"]]).control(
            page.drawer
                .application_background
                .picker_button(
                    |_| Message::DrawerOpen(ContextView::ApplicationBackground),
                    Some(24),
                )
                .width(Length::Fixed(48.0))
                .height(Length::Fixed(24.0)),
        ),
    )
    .on_press(Message::DrawerOpen(ContextView::ApplicationBackground))
}

fn control_tint<'a>(
    page: &Page,
    section: &'a Section<crate::pages::Message>,
    labels: &HashMap<String, usize>,
) -> list::ListButton<'a, Message> {
    let descriptions = &section.descriptions;

    list::button(
        settings::item::builder(&descriptions[labels["control_tint"]])
            .description(&descriptions[labels["control_tint_desc"]])
            .control(
                page.drawer
                    .control_component
                    .picker_button(
                        |_| Message::DrawerOpen(ContextView::ControlComponent),
                        Some(24),
                    )
                    .width(Length::Fixed(48.0))
                    .height(Length::Fixed(24.0)),
            ),
    )
    .on_press(Message::DrawerOpen(ContextView::ControlComponent))
}

fn interface_text<'a>(
    page: &Page,
    section: &'a Section<crate::pages::Message>,
    labels: &HashMap<String, usize>,
) -> list::ListButton<'a, Message> {
    let descriptions = &section.descriptions;

    list::button(
        settings::item::builder(&descriptions[labels["text_tint"]])
            .description(&descriptions[labels["text_tint_desc"]])
            .control(
                page.drawer
                    .interface_text
                    .picker_button(
                        |_| Message::DrawerOpen(ContextView::InterfaceText),
                        Some(24),
                    )
                    .width(Length::Fixed(48.0))
                    .height(Length::Fixed(24.0)),
            ),
    )
    .on_press(Message::DrawerOpen(ContextView::InterfaceText))
}
fn auto_switch<'a>(
    page: &Page,
    section: &'a Section<crate::pages::Message>,
    labels: &HashMap<String, usize>,
) -> list::ListButton<'a, Message> {
    let descriptions = &section.descriptions;

    settings::item::builder(&descriptions[labels["auto_switch"]])
        .description(
            if !page.day_time && page.theme_manager.mode().is_dark {
                &descriptions[labels["auto_switch_desc/sunrise"]]
            } else if page.day_time && !page.theme_manager.mode().is_dark {
                &descriptions[labels["auto_switch_desc/sunset"]]
            } else if page.day_time && page.theme_manager.mode().is_dark {
                &descriptions[labels["auto_switch_desc/next-sunrise"]]
            } else {
                &descriptions[labels["auto_switch_desc/next-sunset"]]
            }
            .clone(),
        )
        .toggler(page.theme_manager.mode().auto_switch, Message::Autoswitch)
}

fn accent_color_palette<'a>(
    page: &Page,
    section: &'a Section<crate::pages::Message>,
    labels: &HashMap<String, usize>,
) -> impl Into<Element<'a, Message>> {
    let descriptions = &section.descriptions;
    let accent = page.theme_manager.accent_palette().as_ref().unwrap();
    let cur_accent = page.theme_manager.builder().accent.map_or_else(
        || page.theme_manager.builder().palette.as_ref().accent_blue,
        Srgba::from,
    );
    let mut accent_palette_row = Vec::with_capacity(accent.len());

    for &color in accent {
        accent_palette_row.push(color_button(
            Some(Message::PaletteAccent(color.into())),
            color.into(),
            cur_accent == color,
            16,
            16,
        ));
    }

    accent_palette_row.push(
        if let Some(c) = page.drawer.custom_accent.get_applied_color() {
            container(color_button(
                Some(Message::DrawerOpen(ContextView::CustomAccent)),
                c,
                cosmic::iced::Color::from(cur_accent) == c,
                16,
                16,
            ))
        } else {
            container(
                page.drawer
                    .custom_accent
                    .picker_button(|_| Message::DrawerOpen(ContextView::CustomAccent), Some(16))
                    .width(Length::Fixed(20.0))
                    .height(Length::Fixed(20.0)),
            )
        }
        .into(),
    );

    settings::item(
        &descriptions[labels["accent_color"]],
        widget::flex_row(accent_palette_row).spacing(9),
    )
}

fn theme_mode<'a>(
    page: &Page,
    section: &'a Section<crate::pages::Message>,
    labels: &HashMap<String, usize>,
) -> impl Into<Element<'a, Message>> {
    let descriptions = &section.descriptions;
    let dark_mode_illustration = widget::icon::from_svg_bytes(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../resources/icons/scalable/status/illustration-appearance-mode-dark.svg"
        ))
        .as_slice(),
    );
    let light_mode_illustration = widget::icon::from_svg_bytes(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../resources/icons/scalable/status/illustration-appearance-mode-light.svg"
        ))
        .as_slice(),
    );

    settings::item(
        fl!("appearance"),
        cosmic::iced::widget::row![
            cosmic::iced::widget::column![
                button::custom_image_button(
                    icon(dark_mode_illustration)
                        .content_fit(ContentFit::Contain)
                        .width(Length::Fill)
                        .height(Length::Fixed(44.0)),
                    None
                )
                .class(button::ButtonClass::Image)
                .selected(page.theme_manager.mode().is_dark)
                .on_press(super::Message::DarkMode(true))
                .padding(1)
                .apply(widget::container)
                .max_width(68),
                text::caption(&descriptions[labels["dark"]])
            ]
            .spacing(8)
            .width(Length::Fixed(68.0))
            .align_x(Alignment::Center),
            cosmic::iced::widget::column![
                button::custom_image_button(
                    icon(light_mode_illustration)
                        .content_fit(ContentFit::Contain)
                        .width(Length::Fill)
                        .height(Length::Fixed(44.0)),
                    None
                )
                .class(button::ButtonClass::Image)
                .selected(!page.theme_manager.mode().is_dark)
                .on_press(super::Message::DarkMode(false))
                .padding(1)
                .apply(widget::container)
                .max_width(68),
                text::caption(&descriptions[labels["light"]])
            ]
            .spacing(8)
            .width(Length::Fixed(68.0))
            .align_x(Alignment::Center)
        ]
        .spacing(18)
        .align_y(Alignment::Center),
    )
}

pub fn color_button<'a, Message: 'a + Clone>(
    on_press: Option<Message>,
    color: cosmic::iced::Color,
    selected: bool,
    width: u16,
    height: u16,
) -> Element<'a, Message> {
    button::custom(widget::space().width(width).height(height))
        .padding(0)
        .selected(selected)
        .class(crate::theme::color_swatch(color, selected))
        .on_press_maybe(on_press)
        .width(Length::Fixed(f32::from(width)))
        .height(Length::Fixed(f32::from(height)))
        .into()
}

#[inline]
fn i18n() -> (slab::Slab<String>, HashMap<String, usize>) {
    let mut descriptions = slab::Slab::new();
    let keys: HashMap<String, usize> = HashMap::from([
        ("auto".into(), descriptions.insert(fl!("auto"))),
        (
            "auto_switch".into(),
            descriptions.insert(fl!("auto-switch")),
        ),
        (
            "auto_switch_desc/sunrise".into(),
            descriptions.insert(fl!("auto-switch", "sunrise")),
        ),
        (
            "auto_switch_desc/sunset".into(),
            descriptions.insert(fl!("auto-switch", "sunset")),
        ),
        (
            "auto_switch_desc/next-sunrise".into(),
            descriptions.insert(fl!("auto-switch", "next-sunrise")),
        ),
        (
            "auto_switch_desc/next-sunset".into(),
            descriptions.insert(fl!("auto-switch", "next-sunset")),
        ),
        (
            "accent_color".into(),
            descriptions.insert(fl!("accent-color")),
        ),
        ("app_bg".into(), descriptions.insert(fl!("app-background"))),
        (
            "container_bg".into(),
            descriptions.insert(fl!("container-background")),
        ),
        (
            "container_bg_desc".into(),
            descriptions.insert(fl!("container-background", "desc")),
        ),
        ("text_tint".into(), descriptions.insert(fl!("text-tint"))),
        (
            "text_tint_desc".into(),
            descriptions.insert(fl!("text-tint", "desc")),
        ),
        (
            "control_tint".into(),
            descriptions.insert(fl!("control-tint")),
        ),
        (
            "control_tint_desc".into(),
            descriptions.insert(fl!("control-tint", "desc")),
        ),
        (
            "window_hint_toggle".into(),
            descriptions.insert(fl!("window-hint-accent-toggle")),
        ),
        (
            "window_hint".into(),
            descriptions.insert(fl!("window-hint-accent")),
        ),
        ("dark".into(), descriptions.insert(fl!("dark"))),
        ("light".into(), descriptions.insert(fl!("light"))),
    ]);

    (descriptions, keys)
}
