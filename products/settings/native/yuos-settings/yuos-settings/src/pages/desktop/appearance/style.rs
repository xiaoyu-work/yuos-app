use cosmic::cosmic_theme::Density;
use cosmic::widget::settings;
use cosmic::{Apply, Element};
use yuos_settings_page::Section;

use super::{Message, Page, Roundness};

pub fn section() -> Section<crate::pages::Message> {
    crate::slab!(descriptions {
        corners = fl!("settings-corners");
        density = fl!("interface-density");
        round = fl!("style", "round");
        slightly_round = fl!("style", "slightly-round");
        square = fl!("style", "square");
        compact = fl!("interface-density", "compact");
        comfortable = fl!("interface-density", "comfortable");
        spacious = fl!("interface-density", "spacious");
    });

    Section::default()
        .title(fl!("style"))
        .descriptions(descriptions)
        .view::<Page>(move |_binder, page, section| {
            let descriptions = &section.descriptions;

            crate::widget::yuos_section()
                .title(&section.title)
                .add(settings::item(
                    &descriptions[corners],
                    crate::widget::segmented_control([
                        (
                            descriptions[round].as_str(),
                            matches!(page.roundness, Roundness::Round),
                            Message::Roundness(Roundness::Round),
                        ),
                        (
                            descriptions[slightly_round].as_str(),
                            matches!(page.roundness, Roundness::SlightlyRound),
                            Message::Roundness(Roundness::SlightlyRound),
                        ),
                        (
                            descriptions[square].as_str(),
                            matches!(page.roundness, Roundness::Square),
                            Message::Roundness(Roundness::Square),
                        ),
                    ]),
                ))
                .add(settings::item(
                    &descriptions[density],
                    crate::widget::segmented_control([
                        (
                            descriptions[compact].as_str(),
                            page.density == Density::Compact,
                            Message::Density(Density::Compact),
                        ),
                        (
                            descriptions[comfortable].as_str(),
                            page.density == Density::Standard,
                            Message::Density(Density::Standard),
                        ),
                        (
                            descriptions[spacious].as_str(),
                            page.density == Density::Spacious,
                            Message::Density(Density::Spacious),
                        ),
                    ]),
                ))
                .apply(Element::from)
                .map(crate::pages::Message::Appearance)
        })
}
