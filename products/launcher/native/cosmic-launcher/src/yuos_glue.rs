// SPDX-License-Identifier: GPL-3.0-only

pub fn ask_yu(query: &str) -> Result<(), yuos_runtime::ask_yu::LaunchError> {
    yuos_runtime::ask_yu::launch_query(query)
}

pub async fn open_desktop_entry(id: &str) -> Result<(), String> {
    let response = yuos_sdk::yu_call_json_async_with_binary(
        "/usr/local/bin/yu",
        "desktop",
        "launch",
        ["__desktop-menu", "run", id],
    )
    .await
    .map_err(|error| error.to_string())?;
    if response["status"] != "submitted" {
        return Err("Yu OS did not confirm opening the application.".into());
    }
    Ok(())
}
