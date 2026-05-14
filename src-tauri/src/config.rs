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
    let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("jackynet");
    p.push("config.json");
    p
}

fn read_or_default(path: &PathBuf) -> AppConfig {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
