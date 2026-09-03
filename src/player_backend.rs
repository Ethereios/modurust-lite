//! Player backend scaffolding
//!
//! Provides a lightweight track player abstraction focused on transport data
//! emission (no actual audio output). It can produce LTC-like frame counters
//! for integration with external systems.

use crate::daw_core::TrackDatabaseEntry;

#[derive(Debug, Clone, Copy)]
pub enum TimecodeFps {
    Fps2997,
    Fps30,
    Fps60,
    Ndf30, // Non-drop-frame example
}

#[derive(Debug, Clone)]
pub struct Timecode {
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
    pub frames: u8,
}

pub fn timecode_from_samples(sample_rate: u32, samples: u64, fps: TimecodeFps) -> Timecode {
    let fps_f = match fps {
        TimecodeFps::Fps2997 => 29.97,
        TimecodeFps::Fps30 => 30.0,
        TimecodeFps::Fps60 => 60.0,
        TimecodeFps::Ndf30 => 30.0,
    };
    let seconds = samples as f64 / sample_rate as f64;
    let total_frames = (seconds * fps_f).floor() as u64;

    let frames = (total_frames % fps_f as u64) as u8;
    let total_seconds = (total_frames as f64 / fps_f).floor() as u64;
    let secs = (total_seconds % 60) as u8;
    let mins = ((total_seconds / 60) % 60) as u8;
    let hours = ((total_seconds / 3600) % 24) as u8;

    Timecode { hours, minutes: mins, seconds: secs, frames }
}

pub struct PlayerBackend {
    pub current_track: Option<TrackDatabaseEntry>,
    pub fps: TimecodeFps,
}

impl PlayerBackend {
    pub fn new() -> Self {
        Self { current_track: None, fps: TimecodeFps::Fps30 }
    }

    pub fn load_track(&mut self, entry: TrackDatabaseEntry) { self.current_track = Some(entry); }

    pub fn unload(&mut self) { self.current_track = None; }

    pub fn ltc_timecode(&self, sample_rate: u32, position_samples: u64) -> Option<Timecode> {
        Some(timecode_from_samples(sample_rate, position_samples, self.fps))
    }
}