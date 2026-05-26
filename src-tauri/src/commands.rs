use std::sync::Arc;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use tauri::Emitter;
use tauri_plugin_autostart::ManagerExt;

use crate::alerts::AlertsState;
use crate::config::{Alert, AppConfig, ConfigStore};
use crate::iface_stats::{IfaceSession, SessionState};
use crate::nethogs::{self, NethogsState, ProcessRow};
use crate::vnstat::{self, InterfaceUsage};

#[tauri::command]
pub fn get_config(store: State<'_, Arc<ConfigStore>>) -> AppConfig {
    store.get()
}

#[tauri::command]
pub fn save_config(app: AppHandle, config: AppConfig, store: State<'_, Arc<ConfigStore>>) -> Result<(), String> {
    let want_autostart = config.autostart;
    store.save(config).map_err(|e| e.to_string())?;
    let manager = app.autolaunch();
    let _ = if want_autostart { manager.enable() } else { manager.disable() };
    Ok(())
}

#[tauri::command]
pub fn set_alias(kind: String, key: String, name: String, store: State<'_, Arc<ConfigStore>>) -> Result<(), String> {
    store.set_alias(&kind, key, name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn nethogs_snapshot(state: State<'_, Arc<NethogsState>>) -> Result<Vec<ProcessRow>, String> {
    Ok(state.snapshot().await)
}

#[tauri::command]
pub async fn get_session_stats(state: State<'_, Arc<SessionState>>) -> Result<Vec<IfaceSession>, String> {
    Ok(state.current().await)
}

#[tauri::command]
pub async fn reset_iface_session(
    name: String,
    state: State<'_, Arc<SessionState>>,
    app: tauri::AppHandle,
) -> Result<Vec<IfaceSession>, String> {
    state.reset(&name).await;
    let stats = state.current().await;
    let _ = app.emit("session:update", stats.clone());
    Ok(stats)
}

#[tauri::command]
pub fn add_alert(alert: Alert, store: State<'_, Arc<ConfigStore>>) -> Result<(), String> {
    store.add_alert(alert).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_alert(
    id: String,
    store: State<'_, Arc<ConfigStore>>,
    alerts: State<'_, Arc<AlertsState>>,
) -> Result<(), String> {
    store.remove_alert(&id).map_err(|e| e.to_string())?;
    alerts.reset(&id).await;
    Ok(())
}

#[tauri::command]
pub async fn toggle_alert_pause(
    id: String,
    store: State<'_, Arc<ConfigStore>>,
    alerts: State<'_, Arc<AlertsState>>,
) -> Result<bool, String> {
    let new_state = store.toggle_alert_pause(&id).map_err(|e| e.to_string())?;
    alerts.reset(&id).await;
    Ok(new_state)
}

#[tauri::command]
pub async fn dismiss_alert(
    id: String,
    alerts: State<'_, Arc<AlertsState>>,
) -> Result<(), String> {
    alerts.dismiss(&id).await;
    // The next periodic tick (~1s) will mark this alert as completed,
    // auto-pause it, and emit the updated alerts:active. The frontend
    // optimistically removes the indicator on click so the UI feels
    // immediate.
    Ok(())
}

#[tauri::command]
pub fn set_alert_iface(name: Option<String>, store: State<'_, Arc<ConfigStore>>) -> Result<(), String> {
    store.set_alert_iface(name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_nethogs_stream(app: AppHandle, state: State<'_, Arc<NethogsState>>) -> Result<(), String> {
    nethogs::start_stream(app, (*state).clone()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_vnstat() -> Result<Vec<InterfaceUsage>, String> {
    vnstat::fetch().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn kill_process(pid: i32) -> Result<(), String> {
    if pid <= 1 { return Err("refusing to kill pid <= 1".into()); }

    // Escalate through four attempts:
    //   1. SIGTERM as user — clean shutdown, works for self-owned processes.
    //   2. SIGTERM via pkexec — same signal, root privileges.
    //   3. SIGKILL as user — process can't ignore it.
    //   4. SIGKILL via pkexec — last resort for root-owned processes.
    let pid_str = pid.to_string();
    let attempts: [(&str, bool); 4] = [
        ("-TERM", false),
        ("-TERM", true),
        ("-KILL", false),
        ("-KILL", true),
    ];

    for (signal, use_pkexec) in attempts.iter() {
        let mut cmd = if *use_pkexec {
            let mut c = std::process::Command::new("pkexec");
            c.args(["/usr/bin/kill", signal, pid_str.as_str()]);
            c
        } else {
            let mut c = std::process::Command::new("/usr/bin/kill");
            c.args([*signal, pid_str.as_str()]);
            c
        };
        let _ = cmd.status();

        // Give the kernel a moment to reap. SIGKILL is immediate but
        // /proc/<pid> may linger briefly while the process is cleaned up.
        std::thread::sleep(std::time::Duration::from_millis(250));
        if !process_alive(pid) { return Ok(()); }
    }

    Err(format!(
        "Could not terminate PID {pid} after SIGTERM and SIGKILL attempts. \
         It may be a kernel thread or stuck in uninterruptible sleep."
    ))
}

fn process_alive(pid: i32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

#[tauri::command]
pub async fn toggle_pip(app: AppHandle, store: State<'_, Arc<ConfigStore>>) -> Result<bool, String> {
    let current = store.get().pip_enabled;
    let new_state = !current;
    store.set_pip(new_state).map_err(|e| e.to_string())?;
    apply_pip(&app, new_state).map_err(|e| e.to_string())?;
    Ok(new_state)
}

pub fn apply_pip(app: &AppHandle, enabled: bool) -> tauri::Result<()> {
    let main = app.get_webview_window("main");
    let pip = app.get_webview_window("pip");

    if enabled {
        if let Some(w) = &main { let _ = w.hide(); }
        if pip.is_none() {
            WebviewWindowBuilder::new(app, "pip", WebviewUrl::App("index.html?view=pip".into()))
                .title("Tauri x25")
                .inner_size(220.0, 330.0)
                .min_inner_size(220.0, 140.0)
                .always_on_top(true)
                .decorations(false)
                .resizable(true)
                .skip_taskbar(true)
                .transparent(true)
                .build()?;
        } else if let Some(w) = pip {
            let _ = w.show();
            let _ = w.set_focus();
        }
    } else {
        if let Some(w) = pip { let _ = w.close(); }
        if let Some(w) = main { let _ = w.show(); let _ = w.set_focus(); }
    }
    Ok(())
}

pub fn toggle_main_visibility(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        match w.is_visible() {
            Ok(true) => { let _ = w.hide(); }
            _ => { let _ = w.show(); let _ = w.set_focus(); }
        }
    }
}
