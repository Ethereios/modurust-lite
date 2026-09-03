//! Global transport and UDP sync broadcaster
//!
//! Implements a simple global transport engine and a UDP-based sync protocol
//! to disseminate beatgrid, transport position, and waveforms. The wire format
//! is JSON for simplicity.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;

use crate::daw_core::{BeatGrid, TrackDatabaseEntry};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportState {
    pub playing: bool,
    pub sample_rate: u32,
    pub position_samples: u64,
    pub tempo_bpm: f32,
    pub pitch_semitones: f32,
    pub time_signature_num: u8,
    pub time_signature_den: u8,
}

pub struct GlobalTransport {
    pub state: TransportState,
    last_tick: Instant,
}

impl GlobalTransport {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            state: TransportState {
                playing: false,
                sample_rate,
                position_samples: 0,
                tempo_bpm: 120.0,
                pitch_semitones: 0.0,
                time_signature_num: 4,
                time_signature_den: 4,
            },
            last_tick: Instant::now(),
        }
    }

    pub fn start(&mut self) {
        self.state.playing = true;
        self.last_tick = Instant::now();
    }

    pub fn stop(&mut self) {
        self.state.playing = false;
    }

    pub fn seek_samples(&mut self, pos: u64) {
        self.state.position_samples = pos;
        self.last_tick = Instant::now();
    }

    pub fn set_tempo(&mut self, bpm: f32) { self.state.tempo_bpm = bpm.max(1.0); }

    pub fn tick(&mut self) {
        let now = Instant::now();
        if self.state.playing {
            let dt = now.duration_since(self.last_tick).as_secs_f64();
            // Advance by wall-clock; for sample-accurate sync a host clock should be used
            let advance = (dt * self.state.sample_rate as f64) as u64;
            self.state.position_samples = self.state.position_samples.saturating_add(advance);
        }
        self.last_tick = now;
    }
}

/// UDP sync broadcaster. Can broadcast both global and per-track messages.
pub struct SyncBroadcaster {
    socket: UdpSocket,
    target: SocketAddr,
}

impl SyncBroadcaster {
    pub async fn new(bind: SocketAddr, target: SocketAddr) -> Result<Self, Box<dyn std::error::Error>> {
        let socket = UdpSocket::bind(bind).await?;
        socket.connect(target).await?;
        Ok(Self { socket, target })
    }

    pub async fn send_transport(&self, state: &TransportState) -> Result<(), Box<dyn std::error::Error>> {
        let payload = serde_json::to_vec(&serde_json::json!({
            "type": "transport_state",
            "state": state,
            "timestamp": chrono::Utc::now().timestamp_millis(),
        }))?;
        self.socket.send(&payload).await?;
        Ok(())
    }

    pub async fn send_beatgrid(&self, track_id: u64, grid: &BeatGrid) -> Result<(), Box<dyn std::error::Error>> {
        let payload = serde_json::to_vec(&serde_json::json!({
            "type": "beatgrid",
            "track_id": track_id,
            "grid": grid,
        }))?;
        self.socket.send(&payload).await?;
        Ok(())
    }

    pub async fn send_track_db(&self, entry: &TrackDatabaseEntry) -> Result<(), Box<dyn std::error::Error>> {
        let payload = serde_json::to_vec(&serde_json::json!({
            "type": "track_db",
            "entry": crate::daw_core::serialize_track_db(entry),
        }))?;
        self.socket.send(&payload).await?;
        Ok(())
    }

    pub async fn send_waveform_chunk(&self, track_id: u64, chunk_index: u32, data: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
        let payload = serde_json::to_vec(&serde_json::json!({
            "type": "waveform_chunk",
            "track_id": track_id,
            "chunk_index": chunk_index,
            "data": data,
        }))?;
        self.socket.send(&payload).await?;
        Ok(())
    }
}

/// Spawn a periodic broadcaster that emits transport and optional beat grid.
pub async fn spawn_sync_task(
    mut transport: GlobalTransport,
    broadcaster: SyncBroadcaster,
    track: Option<TrackDatabaseEntry>,
    interval_ms: u64,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
    let mut sent_grid = false;

    loop {
        interval.tick().await;
        transport.tick();
        let _ = broadcaster.send_transport(&transport.state).await;

        if let Some(ref entry) = track {
            if !sent_grid {
                let _ = broadcaster.send_track_db(entry).await;
                let _ = broadcaster.send_beatgrid(entry.id, &entry.beatgrid).await;
                sent_grid = true;
            }
        }
    }
}