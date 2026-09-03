//! HID/UI to OSC binding registry
//!
//! Provides a simple mapping from control identifiers to OSC-like tags. We use
//! UDP JSON messages for portability in this crate; an external bridge can map
//! these into real OSC if needed.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlBinding {
    pub control_id: String,   // e.g., "play_button", "fader_1"
    pub osc_tag: String,      // e.g., "/transport/play", "/channel/1/volume"
    pub mode: BindingMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BindingMode { Momentary, Toggle, Continuous }

#[derive(Default)]
pub struct BindingRegistry {
    by_control: HashMap<String, ControlBinding>,
    by_osc: HashMap<String, Vec<String>>, // osc->control_ids
}

impl BindingRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register(&mut self, binding: ControlBinding) {
        self.by_osc.entry(binding.osc_tag.clone()).or_default().push(binding.control_id.clone());
        self.by_control.insert(binding.control_id.clone(), binding);
    }

    pub fn lookup_by_control(&self, control_id: &str) -> Option<&ControlBinding> {
        self.by_control.get(control_id)
    }

    pub fn controls_for_osc(&self, osc_tag: &str) -> &[String] {
        self.by_osc.get(osc_tag).map(|v| v.as_slice()).unwrap_or(&[])
    }
}