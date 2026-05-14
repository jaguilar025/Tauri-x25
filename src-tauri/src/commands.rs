use std::sync::Arc;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use tauri_plugin_autostart::ManagerExt;

use crate::config::{AppConfig, ConfigStore};
use crate::nethogs::{self, NethogsState, ProcessRow, SessionStats};
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
pub async fn get_session_stats(state: State<'_, Arc<NethogsState>>) -> Result<SessionStats, String> {
    Ok(state.session_stats().await)
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
    let status = std::process::Command::new("kill")
        .arg("-TERM").arg(pid.to_string())
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        // try pkexec fallback for processes we don't own
        let r = std::process::Command::new("pkexec")
            .args(["kill", "-TERM", &pid.to_string()])
            .status()
            .map_err(|e| e.to_string())?;
        if !r.success() { return Err(format!("kill failed for pid {pid}")); }
    }
    Ok(())
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
                .title("JackyNet")
                .inner_size(320.0, 220.0)
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
