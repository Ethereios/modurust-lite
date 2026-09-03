//! Minimal DAW core primitives (track database, beatgrid, cues, markers)
//!
//! This module intentionally avoids audio/GUI heavy dependencies and focuses on
//! data models and utilities that higher layers (player backend, transport,
//! OSC/MIDI bridge, NUWE nodes) can build upon.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

/// Supported container formats. We constrain to PCM-based WAV/AIFF per spec.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AudioContainerFormat {
    Wav,
    Aiff,
}

/// A colored point in the timeline denoting a cue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CuePoint {
    pub name: String,
    /// Sample position from start of file (not affected by tempo changes)
    pub position_samples: u64,
    /// 0xRRGGBB
    pub color_rgb: u32,
}

/// User markers distinct from performance cues; can hold arbitrary STR data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MarkerData {
    pub name: String,
    pub position_samples: u64,
    pub color_rgb: u32,
    /// Extra side-channel data not related to cue data
    pub extra: serde_json::Value,
}

/// Beat position mapped to an absolute sample position for robust sync
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BeatMarker {
    pub beat_index: u64,
    pub sample_position: u64,
}

/// Adjustable beat grid maintained per track (author utility)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BeatGrid {
    pub bpm: f32,
    /// Optional offset in samples to align the downbeat
    pub offset_samples: i64,
    /// Cached mapping for fast transport sync
    pub markers: Vec<BeatMarker>,
}

impl BeatGrid {
    pub fn compute_markers(&mut self, sample_rate: u32, total_samples: u64) {
        self.markers.clear();
        if self.bpm <= 0.0 || sample_rate == 0 { return; }
        let samples_per_beat = (60.0 / self.bpm) * (sample_rate as f32);
        let mut beat_idx: u64 = 0;
        loop {
            let pos = (beat_idx as f32 * samples_per_beat) as i64 + self.offset_samples;
            if pos < 0 { beat_idx += 1; continue; }
            let pos_u = pos as u64;
            if pos_u > total_samples { break; }
            self.markers.push(BeatMarker { beat_index: beat_idx, sample_position: pos_u });
            beat_idx += 1;
        }
    }
}

/// Extracted, normalized metadata for library/database tagging.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TrackMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub bpm: Option<f32>,
    /// Embedded art if present; frontend may ignore
    pub artwork: Option<Vec<u8>>, // raw bytes
}

/// Track database record combining technical attributes with musical tags.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackDatabaseEntry {
    pub id: u64,
    pub path: PathBuf,
    pub container: AudioContainerFormat,
    pub sample_rate: u32,
    pub channels: u16,
    pub total_samples: u64,
    pub duration_seconds: f32,
    pub metadata: TrackMetadata,
    pub beatgrid: BeatGrid,
    pub cues: Vec<CuePoint>,
    pub markers: Vec<MarkerData>,
    /// Optional downsampled mono waveform for previews/overlays
    pub overview_waveform: Option<Vec<f32>>,
}

impl Default for TrackDatabaseEntry {
    fn default() -> Self {
        Self {
            id: 0,
            path: PathBuf::new(),
            container: AudioContainerFormat::Wav,
            sample_rate: 44100,
            channels: 2,
            total_samples: 0,
            duration_seconds: 0.0,
            metadata: TrackMetadata::default(),
            beatgrid: BeatGrid { bpm: 120.0, offset_samples: 0, markers: Vec::new() },
            cues: Vec::new(),
            markers: Vec::new(),
            overview_waveform: None,
        }
    }
}

/// Loader outcome with minimal PCM facts and optional metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoadedPcmInfo {
    pub container: AudioContainerFormat,
    pub sample_rate: u32,
    pub channels: u16,
    pub total_samples: u64,
}

/// Try to infer container from file path.
fn infer_container(path: &Path) -> Option<AudioContainerFormat> {
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    match ext.as_str() {
        "wav" => Some(AudioContainerFormat::Wav),
        "aif" | "aiff" => Some(AudioContainerFormat::Aiff),
        _ => None,
    }
}

/// Load technical PCM information from a supported file.
/// For WAV we use `hound`. For AIFF we fall back to a light parser for common
/// FORM/COMM chunks; if parsing fails we return a conservative placeholder so
/// higher layers can still function in offline demos.
pub fn load_pcm_info(path: &Path) -> Result<LoadedPcmInfo, Box<dyn std::error::Error>> {
    let container = infer_container(path).ok_or("Unsupported format (expect WAV/AIFF)")?;
    match container {
        AudioContainerFormat::Wav => {
            let reader = hound::WavReader::open(path)?;
            let spec = reader.spec();
            let len = reader.duration();
            Ok(LoadedPcmInfo { container, sample_rate: spec.sample_rate, channels: spec.channels, total_samples: len as u64 })
        }
        AudioContainerFormat::Aiff => {
            // Minimal AIFF parse: read FORM -> COMM for sr/channels/frames
            let mut file = BufReader::new(File::open(path)?);
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            // Naive search for "COMM" chunk
            let comm_pos = buf.windows(4).position(|w| w == b"COMM");
            if let Some(idx) = comm_pos {
                // Big-endian read: channels(2), frames(4), bits(2), sampleRate(10) (80-bit extended)
                if idx + 4 + 18 <= buf.len() {
                    let base = idx + 8; // skip id + size
                    let ch = u16::from_be_bytes([buf[base], buf[base + 1]]);
                    let frames = u32::from_be_bytes([buf[base + 2], buf[base + 3], buf[base + 4], buf[base + 5]]) as u64;
                    // Skip bits
                    // Parse sample rate from 80-bit extended (approximate using top 2 bytes exponent)
                    let exp = ((buf[base + 8] as u16) << 8) | (buf[base + 9] as u16);
                    let mant_high = ((buf[base + 10] as u32) << 24)
                        | ((buf[base + 11] as u32) << 16)
                        | ((buf[base + 12] as u32) << 8)
                        | (buf[base + 13] as u32);
                    // crude 80-bit to f64 approximation
                    let exponent = (exp & 0x7FFF) as i32 - 16383; // remove bias
                    let mut sr = (mant_high as f64) / (1u64 << 31) as f64; // normalize mantissa top 32 bits
                    sr = sr * 2f64.powi(exponent);
                    let sample_rate = sr.round().clamp(8000.0, 192000.0) as u32;
                    return Ok(LoadedPcmInfo { container, sample_rate, channels: ch, total_samples: frames });
                }
            }
            // Fallback if parsing fails
            Ok(LoadedPcmInfo { container, sample_rate: 44100, channels: 2, total_samples: 0 })
        }
    }
}

/// Derive musical metadata. Without optional tag libraries we keep this light
/// and robust: infer fields from filename and allow external injection.
pub fn derive_basic_metadata(path: &Path) -> TrackMetadata {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    // Simple patterns: "Artist - Title (Genre 128bpm)"
    let mut md = TrackMetadata::default();
    if stem.contains(" - ") {
        let parts: Vec<_> = stem.splitn(2, " - ").collect();
        md.artist = Some(parts[0].to_string());
        md.title = Some(parts[1].to_string());
    } else {
        md.title = Some(stem.to_string());
    }
    // crude BPM parse
    if let Some(idx) = stem.to_lowercase().find("bpm") {
        let num: String = stem[..idx].chars().rev().take(4).filter(|c| c.is_ascii_digit() || *c=='.').collect::<String>().chars().rev().collect();
        if let Ok(v) = num.parse::<f32>() { md.bpm = Some(v); }
    }
    md
}

/// Build a `TrackDatabaseEntry` by scanning the file and preparing derived data.
pub fn build_track_entry(id: u64, path: &Path) -> Result<TrackDatabaseEntry, Box<dyn std::error::Error>> {
    let pcm = load_pcm_info(path)?;
    let metadata = derive_basic_metadata(path);
    let duration_seconds = if pcm.sample_rate > 0 { pcm.total_samples as f32 / pcm.sample_rate as f32 } else { 0.0 };
    let mut entry = TrackDatabaseEntry {
        id,
        path: path.to_path_buf(),
        container: pcm.container,
        sample_rate: pcm.sample_rate,
        channels: pcm.channels,
        total_samples: pcm.total_samples,
        duration_seconds,
        metadata,
        ..Default::default()
    };

    // Initialize a default beatgrid and compute markers if bpm is known
    if let Some(bpm) = entry.metadata.bpm { entry.beatgrid.bpm = bpm; }
    entry.beatgrid.compute_markers(entry.sample_rate, entry.total_samples);

    Ok(entry)
}

/// Utility to compute a lightweight mono overview waveform by sparse sampling.
pub fn compute_overview_waveform(samples: &[f32], expected_points: usize) -> Vec<f32> {
    if samples.is_empty() || expected_points == 0 { return Vec::new(); }
    let step = (samples.len() / expected_points.max(1)).max(1);
    let mut out = Vec::with_capacity(expected_points);
    let mut i = 0;
    while i < samples.len() {
        let end = (i + step).min(samples.len());
        let mut peak = 0.0f32;
        for &s in &samples[i..end] { peak = peak.max(s.abs()); }
        out.push(peak);
        i = end;
    }
    out
}

/// Serialize `TrackDatabaseEntry` for network transport (metadata jumbo packet)
pub fn serialize_track_db(entry: &TrackDatabaseEntry) -> serde_json::Value {
    serde_json::json!({
      "id": entry.id,
      "path": entry.path,
      "container": match entry.container { AudioContainerFormat::Wav => "wav", AudioContainerFormat::Aiff => "aiff" },
      "sample_rate": entry.sample_rate,
      "channels": entry.channels,
      "total_samples": entry.total_samples,
      "duration_seconds": entry.duration_seconds,
      "metadata": entry.metadata,
      "beatgrid": {"bpm": entry.beatgrid.bpm, "offset_samples": entry.beatgrid.offset_samples, "markers": entry.beatgrid.markers},
      "cues": entry.cues,
      "markers": entry.markers,
    })
}