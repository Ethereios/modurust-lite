//! NUWE-compatible nodes exposing DAW features

use std::collections::HashMap;
use serde_json::Value;

use crate::daw_core::{build_track_entry, TrackDatabaseEntry};
use crate::transport_sync::{GlobalTransport, SyncBroadcaster, TransportState};
use crate::node_graph::{NeuroNodeProcessor};

pub struct TrackLoaderNode;

#[async_trait::async_trait]
impl NeuroNodeProcessor for TrackLoaderNode {
    async fn process(&mut self, mut inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, String> {
        let path = inputs.remove("path")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or("missing 'path'")?;
        let id = inputs.remove("id").and_then(|v| v.as_u64()).unwrap_or(1);
        let entry = build_track_entry(id, std::path::Path::new(&path))
            .map_err(|e| e.to_string())?;
        let out = serde_json::to_value(&entry).map_err(|e| e.to_string())?;
        Ok(HashMap::from([("track_db".to_string(), out)]))
    }
}

pub struct TransportNode {
    transport: GlobalTransport,
    broadcaster: Option<SyncBroadcaster>,
}

impl TransportNode {
    pub fn new(sample_rate: u32) -> Self { Self { transport: GlobalTransport::new(sample_rate), broadcaster: None } }
}

#[async_trait::async_trait]
impl NeuroNodeProcessor for TransportNode {
    async fn process(&mut self, inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, String> {
        if let Some(v) = inputs.get("tempo") { if let Some(bpm) = v.as_f64() { self.transport.set_tempo(bpm as f32); } }
        if let Some(cmd) = inputs.get("command").and_then(|v| v.as_str()) {
            match cmd { "start" => self.transport.start(), "stop" => self.transport.stop(), _ => {} }
        }
        if let Some(pos) = inputs.get("seek_samples").and_then(|v| v.as_u64()) { self.transport.seek_samples(pos); }
        self.transport.tick();

        // optional broadcasting
        if let Some(b) = self.broadcaster.as_ref() {
            let _ = b.send_transport(&self.transport.state).await;
        }

        let out = serde_json::to_value(&self.transport.state).map_err(|e| e.to_string())?;
        Ok(HashMap::from([("transport_state".to_string(), out)]))
    }
}