use chrono::{DateTime, Local, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::{AlertKind, AppConfig};
use crate::iface_stats::IfaceSession;

#[derive(Debug, Clone, Serialize)]
pub struct ActiveAlert {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Default, Clone)]
struct AlertRuntime {
    triggered_at: Option<DateTime<Local>>,
    dismissed: bool,
}

#[derive(Default)]
pub struct AlertsState {
    /// Per-alert in-memory runtime, keyed by alert id. Not persisted.
    runtime: Mutex<HashMap<String, AlertRuntime>>,
}

impl AlertsState {
    pub fn new() -> Arc<Self> { Arc::new(Self::default()) }

    /// Mark an alert as dismissed — stops the indicator without re-arming
    /// the alert (it stays in `triggered_at` so it won't fire again until
    /// the user pauses/unpauses or modifies it).
    pub async fn dismiss(&self, id: &str) {
        let mut rt = self.runtime.lock().await;
        let entry = rt.entry(id.to_string()).or_default();
        entry.dismissed = true;
    }

    /// Reset all runtime state for an alert (called on pause toggle or
    /// when an alert is removed). Lets a paused-then-unpaused alert
    /// re-evaluate from scratch.
    pub async fn reset(&self, id: &str) {
        let mut rt = self.runtime.lock().await;
        rt.remove(id);
    }

    /// Evaluate all alerts against the current session state. Returns
    /// `(active, completed)` — `active` are alerts to display now,
    /// `completed` are ids of alerts that just finished (either their
    /// notify duration elapsed or they were dismissed by the user) and
    /// should be auto-paused by the caller so they don't re-fire until
    /// the user explicitly resumes them.
    pub async fn evaluate(
        &self,
        config: &AppConfig,
        interfaces: &[IfaceSession],
    ) -> (Vec<ActiveAlert>, Vec<String>) {
        let now = Local::now();
        let iface = pick_iface(config, interfaces);
        let mut rt = self.runtime.lock().await;
        let mut active = Vec::new();
        let mut completed = Vec::new();

        for alert in &config.alerts {
            if alert.paused {
                // Reset transient state when paused so it can fire again
                // when the user un-pauses.
                rt.remove(&alert.id);
                continue;
            }

            let entry = rt.entry(alert.id.clone()).or_default();
            let condition_met = check_condition(&alert.kind, iface);

            // First-time trigger: only fire on rising edge to avoid
            // re-triggering while the condition stays true.
            if entry.triggered_at.is_none() && condition_met {
                entry.triggered_at = Some(now);
                entry.dismissed = false;
            }

            if let Some(t) = entry.triggered_at {
                let expired = match alert.notify_duration.as_str() {
                    "continue" => false,
                    s => {
                        let secs = parse_duration_seconds(s);
                        (now - t).num_seconds() >= secs as i64
                    }
                };

                if entry.dismissed || expired {
                    completed.push(alert.id.clone());
                    continue;
                }

                active.push(ActiveAlert {
                    id: alert.id.clone(),
                    name: alert.name.clone(),
                    color: alert.color.clone(),
                });
            }
        }

        // Drop runtime state for completed alerts so a future un-pause
        // re-evaluates from scratch.
        for id in &completed {
            rt.remove(id);
        }

        (active, completed)
    }
}

fn pick_iface<'a>(config: &AppConfig, interfaces: &'a [IfaceSession]) -> Option<&'a IfaceSession> {
    let up: Vec<&IfaceSession> = interfaces.iter().filter(|i| i.is_up).collect();
    if up.is_empty() {
        return None;
    }
    if let Some(name) = &config.alert_iface {
        if let Some(found) = up.iter().find(|i| &i.name == name) {
            return Some(*found);
        }
    }
    Some(up[0])
}

fn check_condition(kind: &AlertKind, iface: Option<&IfaceSession>) -> bool {
    match kind {
        AlertKind::Chronometer { hms } => {
            let Some(iface) = iface else { return false; };
            let target_secs = parse_hms_seconds(hms);
            let started_ms = iface.started_unix_ms;
            if started_ms == 0 { return false; }
            let elapsed = Utc::now().timestamp_millis() - started_ms;
            elapsed >= (target_secs as i64) * 1000
        }
        AlertKind::Date { iso } => {
            if let Ok(target) = DateTime::parse_from_rfc3339(iso) {
                Local::now() >= target.with_timezone(&Local)
            } else if let Ok(target) = chrono::NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M") {
                let local = target.and_local_timezone(Local).single();
                local.map(|t| Local::now() >= t).unwrap_or(false)
            } else {
                false
            }
        }
        AlertKind::Consumption { bytes, direction } => {
            let Some(iface) = iface else { return false; };
            let current = match direction.as_str() {
                "download" => iface.total_rx_bytes,
                "upload" => iface.total_tx_bytes,
                _ => iface.total_rx_bytes.saturating_add(iface.total_tx_bytes),
            };
            current >= *bytes
        }
    }
}

fn parse_hms_seconds(hms: &str) -> u64 {
    let parts: Vec<&str> = hms.split(':').collect();
    let mut total: u64 = 0;
    for p in &parts {
        total = total.saturating_mul(60).saturating_add(p.parse::<u64>().unwrap_or(0));
    }
    total
}

fn parse_duration_seconds(s: &str) -> u64 {
    match s {
        "10s" => 10,
        "1min" => 60,
        "5min" => 300,
        _ => 60,
    }
}
