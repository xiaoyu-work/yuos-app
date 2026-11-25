// SPDX-License-Identifier: GPL-3.0-only

use std::borrow::Cow;

use crate::widget::text;
use cosmic::widget::{self, button, settings};
use cosmic::{Apply, Task, iced::Length};
use serde::{Deserialize, Serialize};
use slotmap::SlotMap;
use yuos_settings_page::{self as page, Section, section};

use crate::pages;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
enum Request {
    AiBudgetStatus,
    AiBudgetApply { monthly_units: i64 },
}

#[derive(Clone, Debug, Deserialize)]
pub struct Status {
    period: String,
    units_used: i64,
    monthly_units: i64,
}

#[derive(Clone, Debug)]
pub enum Message {
    Refresh,
    Edit(String),
    Save,
    Completed(Result<Status, String>),
}

#[derive(Default)]
pub struct Page {
    confirmed: Option<Status>,
    draft: Option<String>,
    pending: Option<Request>,
    error: Option<String>,
}

async fn request(request: Request) -> Result<Status, String> {
    let bytes = serde_json::to_vec(&request).map_err(|error| error.to_string())?;
    let value = yuos_sdk::yu_call_json_async_with_stdin_binary(
        "/usr/local/bin/yu",
        "ai-budget-settings",
        "control",
        ["__system-settings", "--request-stdin"],
        &bytes,
    )
    .await
    .map_err(|error| error.to_string())?;
    let status: Status = serde_json::from_value(value).map_err(|error| error.to_string())?;
    let valid_period = status.period.split_once('-').is_some_and(|(year, month)| {
        year.len() == 4
            && year.bytes().all(|digit| digit.is_ascii_digit())
            && matches!(month.as_bytes(), [b'0', b'1'..=b'9'] | [b'1', b'0'..=b'2'])
    });
    if !valid_period || status.units_used < 0 || status.monthly_units < 0 {
        return Err("Invalid AI budget status".into());
    }
    if let Request::AiBudgetApply { monthly_units } = request {
        if status.monthly_units != monthly_units {
            return Err("Yu OS did not confirm the requested limit".into());
        }
    }
    Ok(status)
}

fn parse_limit(input: &str) -> Option<i64> {
    let input = input.trim();
    if input.bytes().all(|digit| digit.is_ascii_digit()) {
        input.parse().ok()
    } else {
        None
    }
}

impl page::AutoBind<pages::Message> for Page {}

impl page::Page<pages::Message> for Page {
    fn set_id(&mut self, _: page::Entity) {}

    fn info(&self) -> page::Info {
        page::Info::new("ai-budget", "applications-system-symbolic")
            .title(fl!("ai-budget"))
            .description(fl!("ai-budget-description"))
    }

    fn content(
        &self,
        sections: &mut SlotMap<section::Entity, Section<pages::Message>>,
    ) -> Option<page::Content> {
        let content =
            Section::default().view::<Self>(|_, page, _| page.view().map(pages::Message::AiBudget));
        Some(vec![sections.insert(content)])
    }

    fn on_enter(&mut self) -> Task<pages::Message> {
        self.update(Message::Refresh)
    }
}

impl Page {
    fn start(&mut self, action: Request) -> Task<pages::Message> {
        self.pending = Some(action);
        self.error = None;
        Task::future(
            async move { pages::Message::AiBudget(Message::Completed(request(action).await)) },
        )
    }

    fn changed_limit(&self) -> Option<i64> {
        let confirmed = self.confirmed.as_ref()?;
        parse_limit(self.draft.as_deref()?).filter(|limit| *limit != confirmed.monthly_units)
    }

    pub fn update(&mut self, message: Message) -> Task<pages::Message> {
        match message {
            Message::Refresh if self.pending.is_none() => {
                return self.start(Request::AiBudgetStatus);
            }
            Message::Edit(value) if self.pending.is_none() => self.draft = Some(value),
            Message::Save if self.pending.is_none() => {
                if let Some(monthly_units) = self.changed_limit() {
                    return self.start(Request::AiBudgetApply { monthly_units });
                }
            }
            Message::Completed(result) => {
                if let Some(action) = self.pending.take() {
                    let saving = matches!(action, Request::AiBudgetApply { .. });
                    match result {
                        Ok(status) => {
                            self.confirmed = Some(status);
                            if saving {
                                self.draft = None;
                            }
                        }
                        Err(error) => {
                            self.error = Some(if saving {
                                fl!("ai-budget-save-failed", detail = error.as_str())
                            } else {
                                fl!("ai-budget-load-failed", detail = error.as_str())
                            });
                        }
                    }
                }
            }
            _ => {}
        }
        Task::none()
    }

    fn view(&self) -> cosmic::Element<'_, Message> {
        let ready = self.pending.is_none();
        let mut content = crate::widget::yuos_list_column().add(text::body(fl!("ai-budget-help")));
        if let Some(status) = &self.confirmed {
            let limit = status.monthly_units.to_string();
            let saved_limit = if status.monthly_units == 0 {
                fl!("ai-budget-unlimited")
            } else {
                limit.clone()
            };
            let value = self
                .draft
                .as_deref()
                .map_or(Cow::Owned(limit), Cow::Borrowed);
            let mut input = widget::text_input("", value).width(Length::Fixed(200.0));
            if ready {
                input = input.on_input(Message::Edit);
            }
            content = content
                .add(settings::flex_item(
                    fl!("ai-budget-period"),
                    text::body(&status.period),
                ))
                .add(settings::flex_item(
                    fl!("ai-budget-used"),
                    text::body(status.units_used.to_string()),
                ))
                .add(settings::flex_item(
                    fl!("ai-budget-saved-limit"),
                    text::body(saved_limit),
                ))
                .add(settings::flex_item(fl!("ai-budget-limit"), input));
            if self
                .draft
                .as_deref()
                .is_some_and(|value| parse_limit(value).is_none())
            {
                content = content.add(text::caption(fl!("ai-budget-invalid")));
            }
            content = content.add(button::suggested(fl!("save")).on_press_maybe(
                (ready && self.changed_limit().is_some()).then_some(Message::Save),
            ));
        }
        if let Some(error) = &self.error {
            content = content.add(text::body(error));
        }
        if let Some(action) = self.pending {
            content = content.add(text::caption(match action {
                Request::AiBudgetStatus => fl!("ai-budget-loading"),
                Request::AiBudgetApply { .. } => fl!("ai-budget-saving"),
            }));
        }
        content = content.add(
            button::standard(fl!("ai-budget-refresh"))
                .on_press_maybe(ready.then_some(Message::Refresh)),
        );
        content.into()
    }
}
