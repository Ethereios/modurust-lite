/// Clip-to-Node Integration - Connect arrangement clips to audio nodes
/// 
/// This module implements the critical integration between the Arrangement View clips
/// and the audio processing nodes, allowing clips to trigger and control audio nodes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::node_instance_manager::{NodeInstanceManager, NodeInstanceId};

/// Clip types that can be placed in the arrangement
#[derive(Debug, Clone, PartialEq)]
pub enum ClipType {
    /// Audio clip with sample data
    Audio {
        /// Path to audio file
        file_path: String,
        /// Start offset in the audio file (seconds)
        start_offset: f64,
        /// End offset in the audio file (seconds)
        end_offset: f64,
        /// Playback speed (1.0 = normal)
        speed: f32,
        /// Whether to loop the clip
        loop_enabled: bool,
    },
    /// MIDI clip with note data
    Midi {
        /// MIDI note events
        notes: Vec<MidiNote>,
        /// Target instrument for MIDI data
        target_instrument: Option<String>,
    },
    /// Automation clip with parameter curves
    Automation {
        /// Target parameter path
        target_param: String,
        /// Automation curve points
        points: Vec<(f64, f32)>, // (time, value)
        /// Curve type between points
        curve_type: CurveType,
    },
    /// Pattern clip with sequencer data
    Pattern {
        /// Pattern data
        pattern_data: String,
        /// Pattern type
        pattern_type: String,
    },
}

/// MIDI note event
#[derive(Debug, Clone, PartialEq)]
pub struct MidiNote {
    /// Note number (0-127)
    pub note: u8,
    /// Velocity (0-127)
    pub velocity: u8,
    /// Start time in clip (seconds)
    pub start_time: f64,
    /// Duration (seconds)
    pub duration: f64,
}

/// Curve interpolation type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurveType {
    /// Linear interpolation
    Linear,
    /// Exponential curve
    Exponential,
    /// Logarithmic curve
    Logarithmic,
    /// S-curve (sigmoid)
    SCurve,
    /// Step function (no interpolation)
    Step,
}

/// Clip in the arrangement
#[derive(Debug, Clone)]
pub struct Clip {
    /// Unique clip ID
    pub id: u64,
    /// Clip name
    pub name: String,
    /// Clip type and data
    pub clip_type: ClipType,
    /// Start time in the arrangement (seconds)
    pub start_time: f64,
    /// Duration (seconds)
    pub duration: f64,
    /// Track index
    pub track_index: usize,
    /// Color for UI display
    pub color: (u8, u8, u8),
    /// Whether clip is muted
    pub muted: bool,
    /// Associated node instance (when active)
    pub node_instance: Option<NodeInstanceId>,
}

/// Manager for clip-to-node integration
pub struct ClipNodeIntegration {
    /// Reference to node instance manager
    node_manager: Arc<Mutex<NodeInstanceManager>>,
    /// Map of clip IDs to clips
    clips: HashMap<u64, Clip>,
    /// Map of active clip IDs to their node instances
    active_clips: HashMap<u64, NodeInstanceId>,
    /// Next available clip ID
    next_clip_id: u64,
    /// Current playback position (seconds)
    current_position: f64,
    /// Whether transport is playing
    is_playing: bool,
}

impl ClipNodeIntegration {
    /// Create a new ClipNodeIntegration
    pub fn new(node_manager: Arc<Mutex<NodeInstanceManager>>) -> Self {
        Self {
            node_manager,
            clips: HashMap::new(),
            active_clips: HashMap::new(),
            next_clip_id: 1,
            current_position: 0.0,
            is_playing: false,
        }
    }

    /// Add a new clip to the arrangement
    pub fn add_clip(&mut self, name: &str, clip_type: ClipType, 
                   start_time: f64, duration: f64, track_index: usize) -> u64 {
        let id = self.next_clip_id;
        self.next_clip_id += 1;
        
        let clip = Clip {
            id,
            name: name.to_string(),
            clip_type,
            start_time,
            duration,
            track_index,
            color: (120, 120, 255), // Default blue color
            muted: false,
            node_instance: None,
        };
        
        self.clips.insert(id, clip);
        id
    }
    
    /// Remove a clip from the arrangement
    pub fn remove_clip(&mut self, clip_id: u64) -> bool {
        // Deactivate the clip if it's active
        self.deactivate_clip(clip_id);
        
        // Remove from collection
        self.clips.remove(&clip_id).is_some()
    }
    
    /// Update transport position and process clip triggers
    pub fn update_position(&mut self, position: f64, is_playing: bool) {
        let prev_position = self.current_position;
        let prev_playing = self.is_playing;
        
        self.current_position = position;
        self.is_playing = is_playing;
        
        // If playback just started or position jumped
        if (is_playing && !prev_playing) || (is_playing && (position < prev_position || position > prev_position + 0.1)) {
            self.process_clip_triggers();
        }
        
        // If still playing, check for clip boundaries
        if is_playing {
            self.process_clip_boundaries();
        }
    }
    
    
    
    /// Process clip triggers at the current position
    fn process_clip_triggers(&mut self) {
        let position = self.current_position;
        
        // Gather clip IDs to activate/deactivate to avoid borrow conflicts
        let mut to_activate: Vec<u64> = Vec::new();
        let mut to_deactivate: Vec<u64> = Vec::new();
        for (id, clip) in &self.clips {
            let should_be_active = !clip.muted && position >= clip.start_time && position < clip.start_time + clip.duration;
            let is_active = self.active_clips.contains_key(id);
            if should_be_active && !is_active { to_activate.push(*id); }
            if !should_be_active && is_active { to_deactivate.push(*id); }
        }
        
        for id in to_activate { self.activate_clip(id); }
        for id in to_deactivate { self.deactivate_clip(id); }
    }
    
    /// Get a reference to a clip by ID
    pub fn get_clip(&self, clip_id: u64) -> Option<&Clip> {
        self.clips.get(&clip_id)
    }
    
    /// Get a mutable reference to a clip by ID
    pub fn get_clip_mut(&mut self, clip_id: u64) -> Option<&mut Clip> {
        self.clips.get_mut(&clip_id)
    }
    fn process_clip_boundaries(&mut self) {
        let position = self.current_position;
        
        // Gather starts and ends to avoid borrow conflicts
        let mut to_start: Vec<u64> = Vec::new();
        let mut to_end: Vec<u64> = Vec::new();
        for (id, clip) in &self.clips {
            let should_start = !clip.muted && position >= clip.start_time && position < clip.start_time + 0.1;
            let is_active = self.active_clips.contains_key(id);
            if should_start && !is_active { to_start.push(*id); }
            if is_active && position >= clip.start_time + clip.duration { to_end.push(*id); }
        }
        for id in to_start { self.activate_clip(id); }
        for id in to_end { self.deactivate_clip(id); }
    }
    
    /// Activate a clip by creating its corresponding audio node
    fn activate_clip(&mut self, clip_id: u64) -> bool {
        if let Some(clip) = self.clips.get_mut(&clip_id) {
            if clip.muted {
                return false;
            }
            
            // Create appropriate node based on clip type
            if let Ok(mut node_manager) = self.node_manager.lock() {
                let node_id = match &clip.clip_type {
                    ClipType::Audio { file_path: _file_path, start_offset, end_offset, speed, loop_enabled } => {
                        // Create audio sample player node
                        let node_id = node_manager.create_node(
                            "sample_player", 
                            &format!("Clip: {}", clip.name),
                            (0.0, 0.0) // Position doesn't matter for clip-spawned nodes
                        );
                        
                        // Set node parameters
                        node_manager.set_parameter(node_id, "file_path", 0.0); // Special handling for string params
                        node_manager.set_parameter(node_id, "start_offset", *start_offset as f32);
                        node_manager.set_parameter(node_id, "end_offset", *end_offset as f32);
                        node_manager.set_parameter(node_id, "speed", *speed);
                        node_manager.set_parameter(node_id, "loop_enabled", if *loop_enabled { 1.0 } else { 0.0 });
                        
                        node_id
                    },
                    ClipType::Midi { notes, target_instrument: _ } => {
                        // Create MIDI sequence player node
                        let node_id = node_manager.create_node(
                            "midi_sequencer", 
                            &format!("MIDI: {}", clip.name),
                            (0.0, 0.0)
                        );
                        
                        // Set node parameters - MIDI data would need special handling
                        // This is simplified for the example
                        node_manager.set_parameter(node_id, "note_count", notes.len() as f32);
                        
                        node_id
                    },
                    ClipType::Automation { target_param: _, points, curve_type } => {
                        // Create automation player node
                        let node_id = node_manager.create_node(
                            "automation_player", 
                            &format!("Auto: {}", clip.name),
                            (0.0, 0.0)
                        );
                        
                        // Set node parameters - automation data would need special handling
                        node_manager.set_parameter(node_id, "point_count", points.len() as f32);
                        node_manager.set_parameter(node_id, "curve_type", *curve_type as u8 as f32);
                        
                        node_id
                    },
                    ClipType::Pattern { pattern_data: _, pattern_type: _ } => {
                        // Create pattern player node
                        let node_id = node_manager.create_node(
                            "pattern_player", 
                            &format!("Pattern: {}", clip.name),
                            (0.0, 0.0)
                        );
                        
                        // Set node parameters
                        node_manager.set_parameter(node_id, "pattern_type", 0.0); // Simplified
                        
                        node_id
                    },
                };
                
                // Store node ID in clip and active clips map
                clip.node_instance = Some(node_id);
                self.active_clips.insert(clip_id, node_id);
                
                true
            } else {
                false
            }
        } else {
            false
        }
    }
    
    /// Deactivate a clip by removing its audio node
    fn deactivate_clip(&mut self, clip_id: u64) -> bool {
        if let Some(node_id) = self.active_clips.remove(&clip_id) {
            // Update clip's node instance
            if let Some(clip) = self.clips.get_mut(&clip_id) {
                clip.node_instance = None;
            }
            
            // Remove the node from the audio engine
            if let Ok(mut node_manager) = self.node_manager.lock() {
                node_manager.remove_node(node_id);
                true
            } else {
                false
            }
        } else {
            false
        }
    }
    
    /// Manually trigger a clip (for Live View)
    pub fn trigger_clip(&mut self, clip_id: u64) -> bool {
        if self.active_clips.contains_key(&clip_id) {
            // Already active, do nothing
            true
        } else {
            self.activate_clip(clip_id)
        }
    }
    
    /// Stop a clip (for Live View)
    pub fn stop_clip(&mut self, clip_id: u64) -> bool {
        self.deactivate_clip(clip_id)
    }
    
    /// Get all clips
    pub fn get_clips(&self) -> Vec<&Clip> {
        self.clips.values().collect()
    }
    
    /// Get active clips
    pub fn get_active_clips(&self) -> Vec<&Clip> {
        self.active_clips.keys()
            .filter_map(|id| self.clips.get(id))
            .collect()
    }
}