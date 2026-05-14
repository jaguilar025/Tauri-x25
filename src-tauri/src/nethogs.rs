use anyhow::Result;
use chrono::{DateTime, Local};
use serde::Serialize;
use std::process::Stdio;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct ProcessRow {
    pub program: String,
    pub identity: String,
    pub pid: i32,
    pub user: String,
    pub rx_kbs: f64,
    pub tx_kbs: f64,
    pub cmdline: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionStats {
    pub started_at: String,
    pub started_unix_ms: i64,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
}

#[derive(Default)]
pub struct NethogsState {
    pub latest: Mutex<Vec<ProcessRow>>,
    pub running: Mutex<bool>,
    pub session_start: Mutex<Option<DateTime<Local>>>,
    pub total_rx_bytes: Mutex<u64>,
    pub total_tx_bytes: Mutex<u64>,
}

impl NethogsState {
    pub fn new() -> Arc<Self> { Arc::new(Self::default()) }

    pub async fn snapshot(&self) -> Vec<ProcessRow> {
        self.latest.lock().await.clone()
    }

    pub async fn session_stats(&self) -> SessionStats {
        let start = self.session_start.lock().await.clone();
        let (started_at, started_unix_ms) = match start {
            Some(t) => (t.to_rfc3339(), t.timestamp_millis()),
            None => (String::new(), 0),
        };
        SessionStats {
            started_at,
            started_unix_ms,
            total_rx_bytes: *self.total_rx_bytes.lock().await,
            total_tx_bytes: *self.total_tx_bytes.lock().await,
        }
    }
}

pub async fn start_stream(app: AppHandle, state: Arc<NethogsState>) -> Result<()> {
    {
        let mut running = state.running.lock().await;
        if *running { return Ok(()); }
        *running = true;
    }
    {
        let mut s = state.session_start.lock().await;
        if s.is_none() { *s = Some(Local::now()); }
    }

    tokio::spawn(async move {
        // Always try pkexec first — nethogs needs root to map root-owned
        // processes (e.g. nordvpnd) to PIDs via /proc/<pid>/fd.
        let mut attempt_pkexec = true;
        loop {
            let cmd_result = spawn_nethogs(attempt_pkexec);
            let mut child = match cmd_result {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("nethogs spawn failed: {e}");
                    let _ = app.emit("nethogs:error", format!("{e}"));
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            let stdout = match child.stdout.take() {
                Some(s) => s,
                None => { tokio::time::sleep(std::time::Duration::from_secs(2)).await; continue; }
            };

            let mut reader = BufReader::new(stdout).lines();
            let mut frame: Vec<ProcessRow> = Vec::new();
            let mut last_flush = std::time::Instant::now();

            while let Ok(Some(line)) = reader.next_line().await {
                if let Some(row) = parse_line(&line) {
                    if let Some(existing) = frame.iter_mut().find(|r| r.pid == row.pid && r.identity == row.identity) {
                        *existing = row;
                    } else {
                        frame.push(row);
                    }
                }
                if last_flush.elapsed() >= std::time::Duration::from_millis(900) {
                    frame.sort_by(|a, b| (b.rx_kbs + b.tx_kbs).partial_cmp(&(a.rx_kbs + a.tx_kbs)).unwrap_or(std::cmp::Ordering::Equal));

                    let sum_rx: f64 = frame.iter().map(|r| r.rx_kbs).sum();
                    let sum_tx: f64 = frame.iter().map(|r| r.tx_kbs).sum();
                    {
                        let mut rx = state.total_rx_bytes.lock().await;
                        *rx = rx.saturating_add((sum_rx * 1024.0) as u64);
                    }
                    {
                        let mut tx = state.total_tx_bytes.lock().await;
                        *tx = tx.saturating_add((sum_tx * 1024.0) as u64);
                    }

                    {
                        let mut guard = state.latest.lock().await;
                        *guard = frame.clone();
                    }
                    let _ = app.emit("nethogs:update", frame.clone());
                    let stats = state.session_stats().await;
                    let _ = app.emit("nethogs:session", stats);
                    last_flush = std::time::Instant::now();
                    frame.clear();
                }
            }

            let status = child.wait().await;
            eprintln!("nethogs exited: {:?}", status);
            // If pkexec failed (user cancelled), fall back to plain nethogs
            // so the user at least sees their own processes.
            if attempt_pkexec { attempt_pkexec = false; }
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    });

    Ok(())
}

fn spawn_nethogs(use_pkexec: bool) -> Result<tokio::process::Child> {
    let mut cmd = if use_pkexec {
        let mut c = Command::new("pkexec");
        c.args(["nethogs", "-t", "-d", "1"]);
        c
    } else {
        let mut c = Command::new("nethogs");
        c.args(["-t", "-d", "1"]);
        c
    };
    cmd.stdout(Stdio::piped()).stderr(Stdio::null()).kill_on_drop(true);
    Ok(cmd.spawn()?)
}

fn parse_line(line: &str) -> Option<ProcessRow> {
    let s = line.trim();
    if s.is_empty() || s.starts_with("Refreshing") || s.starts_with("nethogs") { return None; }
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 3 { return None; }
    let tx_kbs: f64 = parts[parts.len()-2].parse().ok()?;
    let rx_kbs: f64 = parts[parts.len()-1].parse().ok()?;
    let ident = parts[..parts.len()-2].join(" ");
    let segments: Vec<&str> = ident.rsplitn(3, '/').collect();
    if segments.len() < 3 { return None; }
    let uid: i32 = segments[0].parse().ok()?;
    let pid: i32 = segments[1].parse().ok()?;
    let program = segments[2].to_string();
    let user = uid_to_user(uid).unwrap_or_else(|| uid.to_string());
    let cmdline = read_cmdline(pid).unwrap_or_default();
    let identity = resolve_identity(&program, pid, &cmdline);
    Some(ProcessRow { program, identity, pid, user, rx_kbs, tx_kbs, cmdline })
}

/// Resolve a stable, discriminative identifier for a process — used as the
/// alias key. nethogs reports `/proc/self/exe` (or paths inside AppImage
/// mounts) for Electron-style apps that re-exec themselves, which collapses
/// distinct apps (Slack, VS Code, Chrome) under the same key. We try
/// `readlink /proc/<pid>/exe` first, then fall back to argv[0] from cmdline.
fn resolve_identity(program: &str, pid: i32, cmdline: &str) -> String {
    if !is_generic_program(program) {
        return program.to_string();
    }

    if let Some(real) = readlink_exe(pid) {
        if !is_generic_program(&real) {
            return real;
        }
    }

    let arg0 = cmdline.split_whitespace().next().unwrap_or("");
    if !arg0.is_empty() && !is_generic_program(arg0) {
        return arg0.to_string();
    }

    program.to_string()
}

fn is_generic_program(p: &str) -> bool {
    p.is_empty()
        || p == "?"
        || p == "unknown"
        || p == "/proc/self/exe"
        || p.starts_with("/tmp/.mount_")
}

fn readlink_exe(pid: i32) -> Option<String> {
    if pid <= 0 { return None; }
    let target = std::fs::read_link(format!("/proc/{pid}/exe")).ok()?;
    let mut s = target.to_string_lossy().into_owned();
    // Kernel appends " (deleted)" when the underlying file has been
    // replaced (e.g. after a package upgrade). Strip it.
    if let Some(stripped) = s.strip_suffix(" (deleted)") {
        s = stripped.to_string();
    }
    if s.is_empty() { None } else { Some(s) }
}

fn uid_to_user(uid: i32) -> Option<String> {
    let contents = std::fs::read_to_string("/etc/passwd").ok()?;
    for line in contents.lines() {
        let cols: Vec<&str> = line.split(':').collect();
        if cols.len() >= 3 && cols[2].parse::<i32>().ok() == Some(uid) {
            return Some(cols[0].to_string());
        }
    }
    None
}

fn read_cmdline(pid: i32) -> Option<String> {
    if pid <= 0 { return None; }
    let raw = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let joined: Vec<u8> = raw.into_iter().map(|b| if b == 0 { b' ' } else { b }).collect();
    Some(String::from_utf8_lossy(&joined).trim().to_string())
}
