// SPDX-License-Identifier: GPL-3.0-only

use std::{path::PathBuf, sync::Arc, time::Duration};

use yuos_sdk::applet;

use crate::{SystemProvider, SystemSummary, Usage};

pub fn provider() -> SystemProvider {
    let client = Client::installed();
    Arc::new(move || {
        let client = client.clone();
        Box::pin(async move { client.system().await.map(system_summary) })
    })
}

#[derive(Clone)]
pub(crate) struct Client {
    binary: PathBuf,
}

impl Client {
    pub(crate) fn installed() -> Self {
        Self { binary: "/usr/local/bin/yu".into() }
    }

    #[cfg(test)]
    pub(crate) fn with_binary(binary: PathBuf) -> Self {
        assert!(binary.is_absolute());
        Self { binary }
    }

    pub(crate) async fn system(&self) -> Result<applet::SystemSummary, String> {
        let value = tokio::time::timeout(
            Duration::from_secs(12),
            yuos_sdk::yu_call_json_async_with_binary(
                &self.binary, "observe", "resources", ["__observe", "resources"],
            ),
        )
        .await
        .map_err(|_| "The OS resource observer timed out.".to_string())?
        .map_err(|error| error.to_string())?;
        serde_json::from_value(value.get("summary").cloned()
            .ok_or("The OS resource observer returned no summary field.")?)
        .map_err(|error| format!("The OS resource observer returned invalid telemetry: {error}"))
    }
}

fn system_summary(summary: applet::SystemSummary) -> SystemSummary {
    let usage = |value: applet::Usage| Usage {
        used_mb: value.used_mb,
        total_mb: value.total_mb,
    };
    SystemSummary {
        cpu_percent: summary.cpu_percent,
        memory: summary.memory.map(usage),
        storage: summary.storage.map(usage),
        network_down_bps: summary.network_down_bps,
        network_up_bps: summary.network_up_bps,
        fallback: summary.fallback,
    }
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/test/unit/service.rs"));
}
