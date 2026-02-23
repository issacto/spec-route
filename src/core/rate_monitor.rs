// src/core/rate_monitor.rs

use crate::config::{RateMonitorConfig, RateMonitorHandle};
use crate::core::worker::Worker;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use dashmap::DashMap;
use crate::core::worker_registry::WorkerId;


pub struct RateMonitor {
    config: RateMonitorConfig,
    slots: Vec<AtomicU64>,
    slot_timestamps: Vec<AtomicU64>,
}

impl RateMonitor {
    pub fn new(config: RateMonitorConfig) -> Self {
        let window = config.window_secs as usize;
        Self {
            slots: (0..window).map(|_| AtomicU64::new(0)).collect(),
            slot_timestamps: (0..window).map(|_| AtomicU64::new(0)).collect(),
            config,
        }
    }

    /// Call this on every incoming request
    pub fn record(&self) -> usize {
        let now = Self::now_secs();
        let idx = (now % self.config.window_secs) as usize;

        // Reset stale slot
        if self.slot_timestamps[idx].load(Ordering::Relaxed) != now {
            self.slots[idx].store(0, Ordering::Relaxed);
            self.slot_timestamps[idx].store(now, Ordering::Relaxed);
        }
        self.slots[idx].fetch_add(1, Ordering::Relaxed);

        self.current_rate(now)
    }

    fn current_rate(&self, now: u64) -> usize {
        self.slots
            .iter()
            .zip(self.slot_timestamps.iter())
            .filter(|(_, ts)| {
                let t = ts.load(Ordering::Relaxed);
                t > 0 && now.saturating_sub(t) < self.config.window_secs
            })
            .map(|(s, _)| s.load(Ordering::Relaxed) as usize)
            .sum()
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    pub fn start(
    monitor: Arc<Self>,
    workers: Arc<DashMap<WorkerId, Arc<dyn Worker>>>,
) -> RateMonitorHandle {
    let handle = tokio::spawn(async move {
        let mut above_since: Option<tokio::time::Instant> = None;

        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let rate = monitor.current_rate(Self::now_secs());
            tracing::info!(rate, "Rate monitor tick");

            let over = rate >= monitor.config.threshold;

            if over {
                let since = above_since.get_or_insert_with(tokio::time::Instant::now);
                if since.elapsed().as_secs() >= monitor.config.sustained_secs {
                    above_since = None;
                    tracing::info!(
                        rate,
                        threshold = monitor.config.threshold,
                        "Rate threshold sustained — checking for speculative workers"
                    );

                    for entry in workers.iter() {
                        let worker = entry.value();
                        if worker.is_healthy() {
                            tracing::info!(url = worker.url(), "Would restart without speculative");
                        }
                    }
                }
            } else {
                above_since = None;
            }
        }
    });

    RateMonitorHandle { handle }
}
}