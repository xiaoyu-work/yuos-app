// SPDX-License-Identifier: GPL-3.0-only

use cosmic::Task;
use yuos_settings_page::{self as page, section, Section};
use slotmap::SlotMap;

use super::common::{self, Modality, State};

#[derive(Clone, Debug)]
pub struct Page {
    entity: page::Entity,
    state: State,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            entity: page::Entity::default(),
            state: State::new(Modality::Decision),
        }
    }
}

impl page::AutoBind<crate::pages::Message> for Page {}

impl page::Page<crate::pages::Message> for Page {
    fn set_id(&mut self, entity: page::Entity) {
        self.entity = entity;
    }

    fn info(&self) -> page::Info {
        page::Info::new(Modality::Decision.page_id(), Modality::Decision.icon_name())
            .title(Modality::Decision.title())
            .description(Modality::Decision.description())
    }

    fn content(
        &self,
        sections: &mut SlotMap<section::Entity, Section<crate::pages::Message>>,
    ) -> Option<page::Content> {
        Some(vec![
            sections.insert(common::status_section::<Page, _>(|p| &p.state, wrap)),
            sections.insert(common::provider_section::<Page, _>(|p| &p.state, wrap)),
            sections.insert(common::actions_section::<Page, _>(|p| &p.state, wrap)),
        ])
    }

    fn on_enter(&mut self) -> Task<crate::pages::Message> {
        self.state.on_enter(wrap)
    }
}

impl Page {
    pub fn update(&mut self, message: common::Message) -> Task<crate::Message> {
        self.state.update(message, wrap).map(Into::into)
    }
}

fn wrap(message: common::Message) -> crate::pages::Message {
    crate::pages::Message::Agent(Modality::Decision, message)
}
