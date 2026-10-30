use crate::Message;
use iced::widget::{column, row, text, slider};
use iced::{Element, Length};

#[derive(Debug, Clone, Copy)]
pub struct Step {
    pub enabled: bool,
    pub velocity: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Lane {
    pub pad_index: usize,
    pub muted: bool,
    pub steps: [Step; 16],
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub lanes: [Lane; 8],
}

#[derive(Debug, Clone)]
pub struct DrumMachine {
    pub playing: bool,
    pub current_step: usize,
    pub tempo: f32,
    pub swing: f32,
    pub fill: bool,
    pub pads: Vec<Lane>,
    pub active: Pattern,
}

impl DrumMachine {
    pub fn new() -> Self {
        let mut pads = Vec::new();
        for i in 0..8 {
            let mut steps = [Step { enabled: false, velocity: 0.8 }; 16];
            if i == 0 { steps[0].enabled = true; }
            if i == 1 { steps[2].enabled = true; }
            if i == 2 { steps[4].enabled = true; }
            if i == 3 { steps[5].enabled = true; }
            if i == 4 { steps[7].enabled = true; }
            if i == 5 { steps[9].enabled = true; }
            if i == 6 { steps[11].enabled = true; }
            if i == 7 { steps[15].enabled = true; }
            pads.push(Lane {
                pad_index: i,
                muted: false,
                steps,
            });
        }
        let lanes = [
            pads[0].clone(), pads[1].clone(), pads[2].clone(), pads[3].clone(),
            pads[4].clone(), pads[5].clone(), pads[6].clone(), pads[7].clone(),
        ];
        Self {
            playing: false,
            current_step: 0,
            tempo: 120.0,
            swing: 0.0,
            fill: false,
            pads,
            active: Pattern { lanes },
        }
    }

    pub fn pad_note(&self, pad: usize) -> u8 {
        let lane = &self.pads[pad];
        36 + lane.pad_index as u8
    }

    pub fn toggle_step(&mut self, lane: usize, step: usize) {
        if lane < self.pads.len() {
            self.pads[lane].steps[step].enabled = !self.pads[lane].steps[step].enabled;
        }
    }

    pub fn set_pattern(&mut self, idx: usize) {
        let _ = idx;
    }

    pub fn active_pattern(&self) -> &Pattern {
        &self.active
    }

    pub fn set_pad_volume(&mut self, pad: usize, v: f32) {
        if pad < self.pads.len() {
            self.pads[pad].steps.iter_mut().for_each(|s| { s.velocity = v.clamp(0.0, 1.0); });
        }
    }

    pub fn set_mute(&mut self, pad: usize, m: bool) {
        if pad < self.pads.len() {
            self.pads[pad].muted = m;
        }
    }

    pub fn advance_step(&mut self, _delta: f32) {
        if self.playing {
            self.current_step = (self.current_step + 1) % 16;
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let play_button = if self.playing {
            iced::widget::Button::new(text("Stop")).on_press(Message::StopSequence)
        } else {
            iced::widget::Button::new(text("Play")).on_press(Message::StartSequence)
        };

        let tempo_slider = slider(60.0_f32..=240.0_f32, self.tempo, Message::SetTempo)
            .step(1.0_f32);

        let controls = row![
            play_button,
            text(format!("Tempo: {:.0} BPM", self.tempo)),
            tempo_slider,
        ]
        .spacing(12);

        let step_buttons: Vec<Element<'_, Message>> = (0..16)
            .map(|step| {
                let is_current = step == self.current_step;
                let is_active = self.pads.iter().any(|lane| lane.steps[step].enabled);
                let label = if is_current {
                    format!("▶{}", step + 1)
                } else if is_active {
                    format!("●{}", step + 1)
                } else {
                    format!("○{}", step + 1)
                };
                iced::widget::Button::new(text(label).size(12))
                    .on_press(Message::StepToggle(0, step))
                    .width(Length::Fill)
                    .into()
            })
            .collect();

        let grid = row![
            column![
                text("Steps:"),
                row(step_buttons).spacing(4),
            ],
            column![
                text("Pads:"),
                column(
                    self.pads.iter().enumerate().map(|(i, lane)| {
                        let note_name = match lane.pad_index {
                            0 => "Kick", 1 => "Snare", 2 => "Hihat", 3 => "Clap",
                            4 => "Tom", 5 => "Cymbal", 6 => "Perc", 7 => "Bass",
                            _ => "Pad",
                        };
                        let btn_text = if lane.muted {
                            format!("{} (M)", note_name)
                        } else {
                            note_name.to_string()
                        };
                        iced::widget::Button::new(text(btn_text).size(12))
                            .on_press(Message::PadPress(i))
                            .width(Length::Fill)
                            .into()
                    })
                ).spacing(4),
            ],
        ]
        .spacing(20);

        column![controls, grid].spacing(16).into()
    }
}