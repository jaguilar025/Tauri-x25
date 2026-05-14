mod commands;
mod config;
mod iface_stats;
mod nethogs;
mod vnstat;

use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::commands::*;
use crate::config::ConfigStore;
use crate::iface_stats::SessionState;
use crate::nethogs::NethogsState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cfg = Arc::new(ConfigStore::load());
    let nethogs_state = NethogsState::new();
    let session_state = SessionState::new();

    let initial_hotkey = cfg.get().hotkey.clone();
    let initial_pip = cfg.get().pip_enabled;

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(cfg.clone())
        .manage(nethogs_state.clone())
        .manage(session_state.clone())
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .setup(move |app| {
            let handle = app.handle().clone();

            // System tray
            let show_i = MenuItem::with_id(app, "show", "Show / Hide Window", true, None::<&str>)?;
            let pip_i = MenuItem::with_id(app, "pip", "Toggle PiP Overlay", true, None::<&str>)?;
            let refresh_i = MenuItem::with_id(app, "refresh", "Refresh", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit JackyNet", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &pip_i, &refresh_i, &quit_i])?;

            let icon = app.default_window_icon().cloned();
            let mut tray_builder = TrayIconBuilder::with_id("main-tray")
                .menu(&menu)
                .tooltip("JackyNet")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => toggle_main_visibility(app),
                        "pip" => {
                            let app = app.clone();
                            tauri::async_runtime::spawn(async move {
                                let store: tauri::State<'_, Arc<ConfigStore>> = app.state();
                                let new_state = !store.get().pip_enabled;
                                let _ = store.set_pip(new_state);
                                let _ = apply_pip(&app, new_state);
                            });
                        }
                        "refresh" => {
                            let _ = app.emit("tray:refresh", ());
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    }
                });
            if let Some(i) = icon { tray_builder = tray_builder.icon(i); }
            tray_builder.build(app)?;

            // Global hotkey: toggle main visibility
            if let Some(shortcut) = parse_shortcut(&initial_hotkey) {
                let h = handle.clone();
                let result = app.global_shortcut().on_shortcut(shortcut, move |_app, _sc, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_main_visibility(&h);
                    }
                });
                if let Err(e) = result {
                    eprintln!("hotkey registration failed: {e}");
                }
            }

            if initial_pip {
                let _ = apply_pip(&handle, true);
            }

            // Periodic interface-counter sampler (every 1s). Reads kernel
            // byte counters from /sys/class/net/*/statistics and emits a
            // per-interface session update to the frontend.
            let session_for_task = session_state.clone();
            let app_for_task = handle.clone();
            tauri::async_runtime::spawn(async move {
                // Prime the initial snapshot so per-iface "started_at" is
                // anchored to app launch, not first event read.
                let _ = session_for_task.tick().await;
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    let stats = session_for_task.tick().await;
                    let _ = app_for_task.emit("session:update", stats);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            set_alias,
            nethogs_snapshot,
            start_nethogs_stream,
            get_vnstat,
            kill_process,
            toggle_pip,
            get_session_stats,
            reset_iface_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running JackyNet");
}

fn parse_shortcut(input: &str) -> Option<Shortcut> {
    let mut mods = Modifiers::empty();
    let mut key: Option<Code> = None;
    for raw in input.split('+') {
        let part = raw.trim();
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "cmdorctrl" | "commandorcontrol" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" | "option" => mods |= Modifiers::ALT,
            "super" | "meta" | "cmd" | "command" => mods |= Modifiers::SUPER,
            other => key = key.or_else(|| code_from_str(other)),
        }
    }
    key.map(|k| Shortcut::new(Some(mods), k))
}

fn code_from_str(s: &str) -> Option<Code> {
    let up = s.to_ascii_uppercase();
    match up.as_str() {
        "A"=>Some(Code::KeyA),"B"=>Some(Code::KeyB),"C"=>Some(Code::KeyC),"D"=>Some(Code::KeyD),
        "E"=>Some(Code::KeyE),"F"=>Some(Code::KeyF),"G"=>Some(Code::KeyG),"H"=>Some(Code::KeyH),
        "I"=>Some(Code::KeyI),"J"=>Some(Code::KeyJ),"K"=>Some(Code::KeyK),"L"=>Some(Code::KeyL),
        "M"=>Some(Code::KeyM),"N"=>Some(Code::KeyN),"O"=>Some(Code::KeyO),"P"=>Some(Code::KeyP),
        "Q"=>Some(Code::KeyQ),"R"=>Some(Code::KeyR),"S"=>Some(Code::KeyS),"T"=>Some(Code::KeyT),
        "U"=>Some(Code::KeyU),"V"=>Some(Code::KeyV),"W"=>Some(Code::KeyW),"X"=>Some(Code::KeyX),
        "Y"=>Some(Code::KeyY),"Z"=>Some(Code::KeyZ),
        "0"=>Some(Code::Digit0),"1"=>Some(Code::Digit1),"2"=>Some(Code::Digit2),"3"=>Some(Code::Digit3),
        "4"=>Some(Code::Digit4),"5"=>Some(Code::Digit5),"6"=>Some(Code::Digit6),"7"=>Some(Code::Digit7),
        "8"=>Some(Code::Digit8),"9"=>Some(Code::Digit9),
        "SPACE"=>Some(Code::Space),"ENTER"=>Some(Code::Enter),"ESCAPE"=>Some(Code::Escape),
        _ => None,
    }
}
