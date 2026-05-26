use chrono::{DateTime, Local};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Snapshot of a single interface at a point in time, sourced from
/// `/sys/class/net/<iface>/` — the kernel's authoritative counters.
#[derive(Debug, Clone)]
pub struct IfaceSnapshot {
    pub name: String,
    pub is_up: bool,
    pub is_virtual: bool,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

/// Per-interface session record. Tracks bytes since the interface was first
/// seen in this Tauri x25 session, plus its current live status.
#[derive(Debug, Clone, Serialize)]
pub struct IfaceSession {
    pub name: String,
    pub is_up: bool,
    pub started_at: String,
    pub started_unix_ms: i64,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
}

/// Mutable in-memory state keyed by interface name. Populated on first tick,
/// updated every second.
#[derive(Default)]
pub struct SessionState {
    pub entries: Mutex<HashMap<String, SessionEntry>>,
}

#[derive(Debug, Clone)]
pub struct SessionEntry {
    pub name: String,
    pub started_at: DateTime<Local>,
    pub initial_rx: u64,
    pub initial_tx: u64,
    pub current_rx: u64,
    pub current_tx: u64,
    pub is_up: bool,
    pub is_virtual: bool,
}

impl SessionState {
    pub fn new() -> Arc<Self> { Arc::new(Self::default()) }

    /// Read all interfaces from /sys and update the in-memory state.
    /// New interfaces start their session timer from this moment. Returns
    /// the active (non-virtual) interfaces ready for the frontend.
    pub async fn tick(&self) -> Vec<IfaceSession> {
        let snaps = read_all();
        let now = Local::now();
        let mut map = self.entries.lock().await;

        for snap in snaps {
            let entry = map.entry(snap.name.clone()).or_insert_with(|| SessionEntry {
                name: snap.name.clone(),
                started_at: now,
                initial_rx: snap.rx_bytes,
                initial_tx: snap.tx_bytes,
                current_rx: snap.rx_bytes,
                current_tx: snap.tx_bytes,
                is_up: snap.is_up,
                is_virtual: snap.is_virtual,
            });
            // If the kernel counter rolled back (interface restarted, driver
            // reload), rebaseline so we don't underflow.
            if snap.rx_bytes < entry.initial_rx { entry.initial_rx = snap.rx_bytes; }
            if snap.tx_bytes < entry.initial_tx { entry.initial_tx = snap.tx_bytes; }
            entry.current_rx = snap.rx_bytes;
            entry.current_tx = snap.tx_bytes;
            entry.is_up = snap.is_up;
            entry.is_virtual = snap.is_virtual;
        }

        map.values()
            .filter(|e| !e.is_virtual)
            .map(|e| IfaceSession {
                name: e.name.clone(),
                is_up: e.is_up,
                started_at: e.started_at.to_rfc3339(),
                started_unix_ms: e.started_at.timestamp_millis(),
                total_rx_bytes: e.current_rx.saturating_sub(e.initial_rx),
                total_tx_bytes: e.current_tx.saturating_sub(e.initial_tx),
            })
            .collect()
    }

    /// Rebaseline a single interface — its accumulated total resets to 0
    /// and `started_at` becomes "now". No-op if the interface isn't tracked.
    pub async fn reset(&self, name: &str) {
        let snaps = read_all();
        let snap = snaps.into_iter().find(|s| s.name == name);
        let now = Local::now();
        let mut map = self.entries.lock().await;
        if let Some(entry) = map.get_mut(name) {
            if let Some(s) = snap {
                entry.initial_rx = s.rx_bytes;
                entry.initial_tx = s.tx_bytes;
                entry.current_rx = s.rx_bytes;
                entry.current_tx = s.tx_bytes;
                entry.is_up = s.is_up;
            } else {
                // Fallback: rebaseline against current cached values.
                entry.initial_rx = entry.current_rx;
                entry.initial_tx = entry.current_tx;
            }
            entry.started_at = now;
        }
    }

    /// Read current state without forcing a refresh from /sys.
    pub async fn current(&self) -> Vec<IfaceSession> {
        let map = self.entries.lock().await;
        map.values()
            .filter(|e| !e.is_virtual)
            .map(|e| IfaceSession {
                name: e.name.clone(),
                is_up: e.is_up,
                started_at: e.started_at.to_rfc3339(),
                started_unix_ms: e.started_at.timestamp_millis(),
                total_rx_bytes: e.current_rx.saturating_sub(e.initial_rx),
                total_tx_bytes: e.current_tx.saturating_sub(e.initial_tx),
            })
            .collect()
    }
}

/// Read all interfaces from `/sys/class/net/*` returning a snapshot per entry.
pub fn read_all() -> Vec<IfaceSnapshot> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/net") else { return out; };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let operstate = read_str(path.join("operstate"));
        let carrier = read_u64(path.join("carrier")).unwrap_or(0);
        let rx_bytes = read_u64(path.join("statistics/rx_bytes")).unwrap_or(0);
        let tx_bytes = read_u64(path.join("statistics/tx_bytes")).unwrap_or(0);

        let is_up = operstate.as_deref() == Some("up") && carrier == 1;
        let is_virtual = is_virtual_iface(&name);

        out.push(IfaceSnapshot { name, is_up, is_virtual, rx_bytes, tx_bytes });
    }
    out
}

fn is_virtual_iface(name: &str) -> bool {
    name == "lo"
        || name.starts_with("docker")
        || name.starts_with("br-")
        || name.starts_with("virbr")
        || name.starts_with("veth")
        || name.starts_with("dummy")
        || name == "bonding_masters"
}

fn read_str(p: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(p).ok().map(|s| s.trim().to_string())
}

fn read_u64(p: impl AsRef<Path>) -> Option<u64> {
    read_str(p).and_then(|s| s.parse::<u64>().ok())
}
