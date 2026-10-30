use crate::Message;
use iced::widget::{column, row, slider, text};
use iced::Element;

#[derive(Debug, Clone)]
pub struct FxRack {
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
    pub filter_mode_idx: usize,
    pub reverb_mix: f32,
    pub delay_time: f32,
    pub delay_feedback: f32,
    pub env_attack: f32,
    pub env_release: f32,
    pub waveform_idx: usize,
    pub master_gain: f32,
}

impl FxRack {
    pub fn new(config: &modurust_core::config::EngineConfig) -> Self {
        Self {
            filter_cutoff: 0.7,
            filter_resonance: 0.5,
            filter_mode_idx: 0,
            reverb_mix: config.nn_sensitivity.clamp(0.0, 1.0),
            delay_time: 0.25,
            delay_feedback: 0.3,
            env_attack: 0.01,
            env_release: 0.3,
            waveform_idx: 0,
            master_gain: 0.8,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let filter_col = column![
            text("Filter"),
            text(format!("Cutoff: {:.0} Hz", self.filter_cutoff * 1000.0)),
            slider(0.0_f32..=1.0_f32, self.filter_cutoff, Message::SetFilterCutoff),
            text(format!("Resonance: {:.2}", self.filter_resonance)),
            slider(0.0_f32..=1.0_f32, self.filter_resonance, Message::SetFilterResonance),
        ];

        let reverb_col = column![
            text("Reverb"),
            text(format!("Mix: {:.2}", self.reverb_mix)),
            slider(0.0_f32..=1.0_f32, self.reverb_mix, Message::SetReverbMix),
        ];

        let delay_col = column![
            text("Delay"),
            text(format!("Time: {:.2}", self.delay_time)),
            slider(0.0_f32..=1.0_f32, self.delay_time, Message::SetDelayTime),
            text(format!("Feedback: {:.2}", self.delay_feedback)),
            slider(0.0_f32..=1.0_f32, self.delay_feedback, Message::SetDelayFeedback),
        ];

        let env_col = column![
            text("Envelope"),
            text(format!("Attack: {:.3}", self.env_attack)),
            slider(0.0_f32..=1.0_f32, self.env_attack, Message::SetEnvAttack),
            text(format!("Release: {:.3}", self.env_release)),
            slider(0.0_f32..=1.0_f32, self.env_release, Message::SetEnvRelease),
        ];

        let synth_col = column![
            text("Synth"),
            text(format!("Master Gain: {:.2}", self.master_gain)),
            slider(0.0_f32..=1.0_f32, self.master_gain, Message::SetMasterGain),
        ];

        row![
            filter_col,
            reverb_col,
            delay_col,
            env_col,
            synth_col,
        ]
        .spacing(20)
        .into()
    }
}
