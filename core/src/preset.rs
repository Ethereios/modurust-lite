use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub version: u32,
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
    pub filter_mode: u32,
    pub reverb_mix: f32,
    pub delay_time: f32,
    pub delay_feedback: f32,
    pub env_attack: f32,
    pub env_release: f32,
    pub waveform: u32,
    pub master_gain: f32,
    pub prime_base: u32,
    pub prime_phase: f32,
    pub nn_sensitivity: f32,
    pub instance_density: u32,
    pub instance_spread: f32,
    pub mode: String,
}

impl Default for Preset {
    fn default() -> Self {
        Self {
            name: "Init".to_string(),
            version: 1,
            filter_cutoff: 0.5,
            filter_resonance: 0.5,
            filter_mode: 0,
            reverb_mix: 0.3,
            delay_time: 0.25,
            delay_feedback: 0.3,
            env_attack: 0.01,
            env_release: 0.3,
            waveform: 1,
            master_gain: 0.8,
            prime_base: 2,
            prime_phase: 0.0,
            nn_sensitivity: 0.5,
            instance_density: 16,
            instance_spread: 0.5,
            mode: "synth".to_string(),
        }
    }
}

impl Preset {
    pub fn from_engine_params(params: &[f32], mode: &str) -> Self {
        Self {
            name: "Current".to_string(),
            version: 1,
            filter_cutoff: params.get(0).copied().unwrap_or(0.5),
            filter_resonance: params.get(1).copied().unwrap_or(0.5),
            filter_mode: params.get(2).copied().unwrap_or(0.0) as u32,
            reverb_mix: params.get(3).copied().unwrap_or(0.3),
            delay_time: params.get(4).copied().unwrap_or(0.25),
            delay_feedback: params.get(5).copied().unwrap_or(0.3),
            env_attack: params.get(6).copied().unwrap_or(0.01),
            env_release: params.get(7).copied().unwrap_or(0.3),
            waveform: params.get(8).copied().unwrap_or(1.0) as u32,
            master_gain: params.get(9).copied().unwrap_or(0.8),
            prime_base: params.get(10).copied().unwrap_or(2.0) as u32,
            prime_phase: params.get(11).copied().unwrap_or(0.0),
            nn_sensitivity: params.get(12).copied().unwrap_or(0.5),
            instance_density: params.get(13).copied().unwrap_or(16.0) as u32,
            instance_spread: params.get(14).copied().unwrap_or(0.5),
            mode: mode.to_string(),
        }
    }

    pub fn to_param_array(&self) -> [f32; 16] {
        [
            self.filter_cutoff,
            self.filter_resonance,
            self.filter_mode as f32,
            self.reverb_mix,
            self.delay_time,
            self.delay_feedback,
            self.env_attack,
            self.env_release,
            self.waveform as f32,
            self.master_gain,
            self.prime_base as f32,
            self.prime_phase,
            self.nn_sensitivity,
            self.instance_density as f32,
            self.instance_spread,
            0.0,
        ]
    }

    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    pub fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_default() {
        let preset = Preset::default();
        assert_eq!(preset.name, "Init");
        assert_eq!(preset.version, 1);
        assert_eq!(preset.filter_cutoff, 0.5);
    }

    #[test]
    fn test_preset_roundtrip() {
        let preset = Preset {
            name: "Test Preset".to_string(),
            version: 1,
            filter_cutoff: 0.75,
            filter_resonance: 0.6,
            filter_mode: 2,
            reverb_mix: 0.4,
            delay_time: 0.3,
            delay_feedback: 0.5,
            env_attack: 0.02,
            env_release: 0.4,
            waveform: 2,
            master_gain: 0.85,
            prime_base: 5,
            prime_phase: 0.25,
            nn_sensitivity: 0.7,
            instance_density: 32,
            instance_spread: 0.6,
            mode: "synth".to_string(),
        };

        let params = preset.to_param_array();
        let from_params = Preset::from_engine_params(&params, &preset.mode);

        assert_eq!(preset.filter_cutoff, from_params.filter_cutoff);
        assert_eq!(preset.filter_resonance, from_params.filter_resonance);
        assert_eq!(preset.mode, from_params.mode);
    }
}
