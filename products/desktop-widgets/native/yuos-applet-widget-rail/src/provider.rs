// SPDX-License-Identifier: GPL-3.0-only

use std::{future::Future, pin::Pin, sync::Arc};

pub type DataFuture<T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send>>;

pub type SystemProvider = Arc<dyn Fn() -> DataFuture<SystemSummary> + Send + Sync>;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SystemSummary {
    pub cpu_percent: Option<f32>,
    pub memory: Option<Usage>,
    pub storage: Option<Usage>,
    pub network_down_bps: Option<u64>,
    pub network_up_bps: Option<u64>,
    pub fallback: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub used_mb: u64,
    pub total_mb: u64,
}

impl Usage {
    pub fn percent(self) -> u64 {
        if self.total_mb == 0 {
            0
        } else {
            self.used_mb.saturating_mul(100) / self.total_mb
        }
    }
}
