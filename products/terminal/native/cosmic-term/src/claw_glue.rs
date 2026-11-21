// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

use yuos_runtime::ask_yu;
use serde::Serialize;

#[derive(Serialize)]
struct TerminalContext<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd: Option<&'a str>,
}

impl ask_yu::Context for TerminalContext<'_> {
    const APP_ID: &'static str = "cosmic-term";
}

#[derive(Serialize)]
struct ExplainOutputContext<'output, 'cwd> {
    mode: &'static str,
    output: &'output str,
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd: Option<&'cwd str>,
    #[serde(skip_serializing_if = "is_false")]
    truncated: bool,
}

impl ask_yu::Context for ExplainOutputContext<'_, '_> {
    const APP_ID: &'static str = "cosmic-term";
}

fn is_false(value: &bool) -> bool {
    !value
}

pub fn ask_yu(cwd: Option<&str>) -> Result<(), ask_yu::LaunchError> {
    ask_yu::launch(&TerminalContext { cwd })
}

pub fn explain_output(output: &str, cwd: Option<&str>) -> Result<(), ask_yu::LaunchError> {
    let context = bounded_explain_context(output, cwd)?;
    ask_yu::launch(&context)
}

fn bounded_explain_context<'output, 'cwd>(
    output: &'output str,
    cwd: Option<&'cwd str>,
) -> Result<ExplainOutputContext<'output, 'cwd>, ask_yu::ContextError> {
    let full = ExplainOutputContext {
        mode: "explain_output",
        output,
        cwd,
        truncated: false,
    };
    if ask_yu::context_fits(&full)? {
        return Ok(full);
    }

    let output = ask_yu::newest_fitting_text_suffix(output, |candidate| {
        ask_yu::context_fits(&ExplainOutputContext {
            mode: "explain_output",
            output: candidate,
            cwd,
            truncated: true,
        })
    })?
    .ok_or(ask_yu::ContextError::NoRoomForText)?;

    Ok(ExplainOutputContext {
        mode: "explain_output",
        output,
        cwd,
        truncated: true,
    })
}

#[cfg(test)]
mod tests {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test/unit/claw_glue.rs"
    ));
}
