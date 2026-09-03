use std::sync::mpsc::{channel, Receiver};
use notify::{Watcher, RecursiveMode, Result, Event};
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub mode: String,
    pub audio_backend: String,
    pub midi_backend: String,
    pub udp_port: u16,
    pub osc_port: u16,
    pub status_port: u16,
    pub prime_base: u32,
    pub prime_phase: f32,
    pub nn_mode: String,
    pub nn_sensitivity: f32,
    pub instance_density: u32,
    pub instance_spread: f32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            mode: "synth".to_string(),
            audio_backend: "cpal".to_string(),
            midi_backend: "midir".to_string(),
            udp_port: 9000,
            osc_port: 9001,
            status_port: 9002,
            prime_base: 2,
            prime_phase: 0.0,
            nn_mode: "complex_phase".to_string(),
            nn_sensitivity: 0.5,
            instance_density: 16,
            instance_spread: 0.5,
        }
    }
}

impl EngineConfig {
    /// Load a config from a JSON file, falling back to defaults on error.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str::<EngineConfig>(&content).ok())
            .unwrap_or_default()
    }

    /// Load from a JSON string, falling back to defaults on error.
    pub fn load_from_str(content: &str) -> Self {
        serde_json::from_str::<EngineConfig>(content).unwrap_or_default()
    }

    /// Serialize to JSON string (pretty-printed).
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Persist this config to a JSON file.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        std::fs::write(path, self.to_json())
    }
}

pub struct ConfigWatcher {
    _watcher: notify::RecommendedWatcher,
    pub rx: Receiver<EngineConfig>,
}

impl ConfigWatcher {
    /// Load the current config once (non-blocking) from disk.
    pub fn watch_once<P: AsRef<Path>>(path: P) -> Option<EngineConfig> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str::<EngineConfig>(&content).ok()
    }

    pub fn watch<P: AsRef<Path>>(path: P) -> Result<Self> {
        let (tx, rx) = channel();
        let config_path = path.as_ref().to_path_buf();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event>| {
            if let Ok(event) = res {
                if event.kind.is_modify() {
                    if let Ok(content) = std::fs::read_to_string(&config_path) {
                        if let Ok(config) = serde_json::from_str::<EngineConfig>(&content) {
                            let _ = tx.send(config);
                        }
                    }
                }
            }
        })?;

        watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;

        Ok(Self { _watcher: watcher, rx })
    }
}
