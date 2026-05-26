use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub interface_aliases: HashMap<String, String>,
    #[serde(default)]
    pub process_aliases: HashMap<String, String>,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default)]
    pub pip_enabled: bool,
    #[serde(default)]
    pub alerts: Vec<Alert>,
    #[serde(default)]
    pub alert_iface: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub name: String,
    pub color: String,
    pub notify_duration: String, // "10s" | "1min" | "5min" | "continue"
    #[serde(flatten)]
    pub kind: AlertKind,
    #[serde(default)]
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AlertKind {
    Chronometer { hms: String },             // "HH:MM:SS"
    Date { iso: String },                    // ISO 8601 datetime
    Consumption {
        bytes: u64,
        direction: String,                   // "download" | "upload" | "combined"
    },
}

fn default_hotkey() -> String { "CmdOrCtrl+Shift+N".to_string() }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            interface_aliases: HashMap::new(),
            process_aliases: HashMap::new(),
            hotkey: default_hotkey(),
            autostart: false,
            pip_enabled: false,
            alerts: Vec::new(),
            alert_iface: None,
        }
    }
}

pub struct ConfigStore {
    pub path: PathBuf,
    pub data: Mutex<AppConfig>,
}

impl ConfigStore {
    pub fn load() -> Self {
        let path = config_path();
        let data = read_or_default(&path);
        Self { path, data: Mutex::new(data) }
    }

    pub fn get(&self) -> AppConfig {
        self.data.lock().unwrap().clone()
    }

    pub fn save(&self, new_cfg: AppConfig) -> Result<()> {
        {
            let mut guard = self.data.lock().unwrap();
            *guard = new_cfg;
        }
        self.persist()
    }

    pub fn set_alias(&self, kind: &str, key: String, name: String) -> Result<()> {
        {
            let mut guard = self.data.lock().unwrap();
            let map = match kind {
                "interface" => &mut guard.interface_aliases,
                "process" => &mut guard.process_aliases,
                _ => return Err(anyhow::anyhow!("unknown alias kind")),
            };
            if name.trim().is_empty() {
                map.remove(&key);
            } else {
                map.insert(key, name);
            }
        }
        self.persist()
    }

    pub fn set_pip(&self, enabled: bool) -> Result<()> {
        {
            let mut guard = self.data.lock().unwrap();
            guard.pip_enabled = enabled;
        }
        self.persist()
    }

    pub fn add_alert(&self, alert: Alert) -> Result<()> {
        {
            let mut guard = self.data.lock().unwrap();
            if guard.alerts.len() >= 10 {
                return Err(anyhow::anyhow!("maximum of 10 alerts reached"));
            }
            if guard.alerts.iter().any(|a| a.id == alert.id) {
                return Err(anyhow::anyhow!("alert id already exists"));
            }
            guard.alerts.push(alert);
        }
        self.persist()
    }

    pub fn remove_alert(&self, id: &str) -> Result<()> {
        {
            let mut guard = self.data.lock().unwrap();
            guard.alerts.retain(|a| a.id != id);
        }
        self.persist()
    }

    pub fn toggle_alert_pause(&self, id: &str) -> Result<bool> {
        let mut new_state = false;
        {
            let mut guard = self.data.lock().unwrap();
            if let Some(a) = guard.alerts.iter_mut().find(|a| a.id == id) {
                a.paused = !a.paused;
                new_state = a.paused;
            }
        }
        self.persist()?;
        Ok(new_state)
    }

    pub fn set_alert_paused(&self, id: &str, paused: bool) -> Result<bool> {
        let mut changed = false;
        {
            let mut guard = self.data.lock().unwrap();
            if let Some(a) = guard.alerts.iter_mut().find(|a| a.id == id) {
                if a.paused != paused {
                    a.paused = paused;
                    changed = true;
                }
            }
        }
        if changed { self.persist()?; }
        Ok(changed)
    }

    pub fn set_alert_iface(&self, iface: Option<String>) -> Result<()> {
        {
            let mut guard = self.data.lock().unwrap();
            guard.alert_iface = iface;
        }
        self.persist()
    }

    fn persist(&self) -> Result<()> {
        let guard = self.data.lock().unwrap();
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let json = serde_json::to_string_pretty(&*guard)?;
        fs::write(&self.path, json)?;
        Ok(())
    }
}

fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let new_dir = base.join("tauri-x25");
    let new_path = new_dir.join("config.json");

    // One-time migration from previous config directory names.
    // Tries the most recent legacy name first, then the older one.
    if !new_path.exists() {
        for legacy_name in ["taurix25", "jackynet"] {
            let legacy_dir = base.join(legacy_name);
            if !legacy_dir.exists() {
                continue;
            }
            if new_dir.exists() {
                let legacy_cfg = legacy_dir.join("config.json");
                if legacy_cfg.exists() {
                    let _ = fs::rename(&legacy_cfg, &new_path);
                }
            } else {
                let _ = fs::rename(&legacy_dir, &new_dir);
            }
            if new_path.exists() {
                break;
            }
        }
    }

    new_path
}

fn read_or_default(path: &PathBuf) -> AppConfig {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
