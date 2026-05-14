use anyhow::{Context, Result};
use serde::Serialize;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct UsageEntry {
    pub date: String,
    pub rx: u64,
    pub tx: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct InterfaceUsage {
    pub name: String,
    pub total_rx: u64,
    pub total_tx: u64,
    pub daily: Vec<UsageEntry>,
    pub monthly: Vec<UsageEntry>,
}

/// Query vnstat JSON output and parse into structured data.
/// `vnstat --json` returns all interfaces with daily/monthly arrays.
pub async fn fetch() -> Result<Vec<InterfaceUsage>> {
    let out = Command::new("vnstat")
        .args(["--json"])
        .output()
        .await
        .context("failed to spawn vnstat (is it installed?)")?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("vnstat exited with error: {}", err.trim());
    }

    let json: serde_json::Value = serde_json::from_slice(&out.stdout)
        .context("failed to parse vnstat JSON")?;

    let mut interfaces = Vec::new();
    let ifaces = json.get("interfaces").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    for iface in ifaces {
        let name = iface.get("name").and_then(|v| v.as_str()).unwrap_or("?").to_string();
        let traffic = iface.get("traffic");
        let total_rx = traffic.and_then(|t| t.get("total")).and_then(|t| t.get("rx")).and_then(|v| v.as_u64()).unwrap_or(0);
        let total_tx = traffic.and_then(|t| t.get("total")).and_then(|t| t.get("tx")).and_then(|v| v.as_u64()).unwrap_or(0);

        let daily = extract_entries(traffic, "day");
        let monthly = extract_entries(traffic, "month");

        interfaces.push(InterfaceUsage { name, total_rx, total_tx, daily, monthly });
    }

    Ok(interfaces)
}

fn extract_entries(traffic: Option<&serde_json::Value>, key: &str) -> Vec<UsageEntry> {
    let arr = traffic
        .and_then(|t| t.get(key))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut out: Vec<UsageEntry> = arr.into_iter().filter_map(|entry| {
        let rx = entry.get("rx").and_then(|v| v.as_u64()).unwrap_or(0);
        let tx = entry.get("tx").and_then(|v| v.as_u64()).unwrap_or(0);
        let date = entry.get("date").map(|d| {
            let y = d.get("year").and_then(|v| v.as_i64()).unwrap_or(0);
            let m = d.get("month").and_then(|v| v.as_i64()).unwrap_or(0);
            let day = d.get("day").and_then(|v| v.as_i64());
            match day {
                Some(dd) => format!("{:04}-{:02}-{:02}", y, m, dd),
                None => format!("{:04}-{:02}", y, m),
            }
        }).unwrap_or_else(|| "?".into());
        Some(UsageEntry { date, rx, tx })
    }).collect();

    out.sort_by(|a, b| b.date.cmp(&a.date));
    out
}
