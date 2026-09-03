/// Node Graph - Audio processing graph management
/// 
/// This module implements the node-based audio processing system that connects
/// DSP modules and manages audio flow, including topological sorting for
/// optimal processing order.

use std::collections::{HashMap, VecDeque};
use super::dsp_core::{DSPModule, Oscillator, Filter, Delay, Reverb};

/// Node types for the audio graph
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Oscillator,
    Filter,
    Delay,
    Reverb,
    Mixer,
    VCA,
    Output,
    Input,
}

/// Connection between nodes
#[derive(Debug, Clone)]
pub struct NodeConnection {
    pub from_node: usize,
    pub to_node: usize,
    pub from_port: String,  // e.g., "audio_out", "cv_out"
    pub to_port: String,    // e.g., "audio_in", "cv_in"
}

/// Individual node in the graph
pub(crate) struct GraphNode {
    id: usize,
    node_type: NodeType,
    parameters: HashMap<String, f32>,
    dsp_module: Option<Box<dyn DSPModule>>,
}

impl std::fmt::Debug for GraphNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphNode")
            .field("id", &self.id)
            .field("node_type", &self.node_type)
            .field("parameters", &self.parameters)
            .field("dsp_module", &self.dsp_module.as_ref().map(|_| "<DSPModule>"))
            .finish()
    }
}

/// Node processing state
#[derive(Debug)]
pub struct NodeState {
    pub output_buffer: Vec<f32>,
    pub input_buffer: Vec<f32>,
    pub parameter_cache: HashMap<String, f32>,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            output_buffer: vec![0.0; 1024],
            input_buffer: vec![0.0; 1024],
            parameter_cache: HashMap::new(),
        }
    }
}

/// Audio flow information for real-time processing
#[derive(Debug, Clone)]
pub struct AudioFlow {
    pub sample_rate: u32,
    pub buffer_size: usize,
    pub current_time: f64,
    pub tempo: f32,
}

/// Main node graph for audio processing
pub struct NodeGraph {
    nodes: HashMap<usize, GraphNode>,
    connections: Vec<NodeConnection>,
    output_node: Option<usize>,
    input_node: Option<usize>,
    master_volume: f32,
    master_pan: f32,
    master_gain_smooth: f32,
    master_pan_smooth: f32,
    track_volumes: Vec<f32>,
    track_gain_smooth: Vec<f32>,
    track_pans: Vec<f32>,
    return_volumes: Vec<f32>,
    return_pans: Vec<f32>,
    audio_flow: AudioFlow,
}

impl NodeGraph {
    /// Create a new empty node graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            connections: Vec::new(),
            output_node: None,
            input_node: None,
            master_volume: 0.8,
            master_pan: 0.0,
            master_gain_smooth: 0.8,
            master_pan_smooth: 0.0,
            track_volumes: vec![0.8; 12], // 12 track volumes
            track_gain_smooth: vec![0.8; 12],
            track_pans: vec![0.0; 12],    // centered pan for tracks
            return_volumes: vec![0.7; 8], // 8 return buses
            return_pans: vec![0.0; 8],    // centered pan for returns
            audio_flow: AudioFlow {
                sample_rate: 44100,
                buffer_size: 256,
                current_time: 0.0,
                tempo: 128.0,
            },
        }
    }
    
    /// Set audio flow parameters
    pub fn set_audio_flow(&mut self, sample_rate: u32, buffer_size: usize) {
        self.audio_flow.sample_rate = sample_rate;
        self.audio_flow.buffer_size = buffer_size;
    }
    
    /// Add an oscillator node
    pub fn add_oscillator(&mut self) -> usize {
        let id = self.nodes.len();
        let mut node = GraphNode {
            id,
            node_type: NodeType::Oscillator,
            parameters: HashMap::new(),
            dsp_module: Some(Box::new(Oscillator::new(self.audio_flow.sample_rate as f32))),
        };
        
        // Set default parameters
        node.parameters.insert("frequency".to_string(), 440.0);
        node.parameters.insert("amplitude".to_string(), 0.8);
        
        self.nodes.insert(id, node);
        id
    }
    
    /// Add a filter node
    pub fn add_filter(&mut self) -> usize {
        let id = self.nodes.len();
        let mut node = GraphNode {
            id,
            node_type: NodeType::Filter,
            parameters: HashMap::new(),
            dsp_module: Some(Box::new(Filter::new(self.audio_flow.sample_rate as f32))),
        };
        
        // Set default parameters
        node.parameters.insert("cutoff".to_string(), 1000.0);
        node.parameters.insert("resonance".to_string(), 0.7);
        
        self.nodes.insert(id, node);
        id
    }
    
    /// Add a delay node
    pub fn add_delay(&mut self) -> usize {
        let id = self.nodes.len();
        let mut node = GraphNode {
            id,
            node_type: NodeType::Delay,
            parameters: HashMap::new(),
            dsp_module: Some(Box::new(Delay::new(self.audio_flow.sample_rate as f32, 2.0))),
        };
        
        // Set default parameters
        node.parameters.insert("delay_time".to_string(), 0.25);
        node.parameters.insert("feedback".to_string(), 0.3);
        node.parameters.insert("mix".to_string(), 0.5);
        
        self.nodes.insert(id, node);
        id
    }
    
    /// Add a reverb node
    pub fn add_reverb(&mut self) -> usize {
        let id = self.nodes.len();
        let mut node = GraphNode {
            id,
            node_type: NodeType::Reverb,
            parameters: HashMap::new(),
            dsp_module: Some(Box::new(Reverb::new(self.audio_flow.sample_rate as f32))),
        };
        
        // Set default parameters
        node.parameters.insert("room_size".to_string(), 1.0);
        node.parameters.insert("mix".to_string(), 0.3);
        node.parameters.insert("damping".to_string(), 0.5);
        
        self.nodes.insert(id, node);
        id
    }
    
    /// Add a mixer node
    pub fn add_mixer(&mut self, _inputs: usize) -> usize {
        let id = self.nodes.len();
        let node = GraphNode {
            id,
            node_type: NodeType::Mixer,
            parameters: HashMap::new(),
            dsp_module: None, // Mixers are handled specially in the graph
        };
        
        self.nodes.insert(id, node);
        id
    }
    
    /// Add an output node
    pub fn add_output(&mut self) -> usize {
        let id = self.nodes.len();
        let node = GraphNode {
            id,
            node_type: NodeType::Output,
            parameters: HashMap::new(),
            dsp_module: None,
        };
        
        self.nodes.insert(id, node);
        self.output_node = Some(id);
        id
    }
    
    /// Add an input node
    pub fn add_input(&mut self) -> usize {
        let id = self.nodes.len();
        let node = GraphNode {
            id,
            node_type: NodeType::Input,
            parameters: HashMap::new(),
            dsp_module: None,
        };
        
        self.nodes.insert(id, node);
        self.input_node = Some(id);
        id
    }
    
    /// Connect two nodes
    pub fn connect(&mut self, from_node: usize, to_node: usize, from_port: &str, to_port: &str) -> Result<(), String> {
        // Validate nodes exist
        if !self.nodes.contains_key(&from_node) {
            return Err(format!("Source node {} does not exist", from_node));
        }
        if !self.nodes.contains_key(&to_node) {
            return Err(format!("Destination node {} does not exist", to_node));
        }
        
        // Create connection
        let connection = NodeConnection {
            from_node,
            to_node,
            from_port: from_port.to_string(),
            to_port: to_port.to_string(),
        };
        
        self.connections.push(connection);
        Ok(())
    }
    
    /// Disconnect nodes
    pub fn disconnect(&mut self, from_node: usize, to_node: usize) {
        self.connections.retain(|conn| conn.from_node != from_node || conn.to_node != to_node);
    }
    
    /// Process the entire audio graph
    pub fn process(&mut self, input_buffer: &[f32], output_buffer: &mut [f32]) {
        if self.nodes.is_empty() {
            output_buffer.copy_from_slice(input_buffer);
            return;
        }
        
        // Get processing order using topological sort
        let processing_order = match self.get_processing_order() {
            Ok(order) => order,
            Err(e) => {
                println!("⚠️ Graph processing order error: {}", e);
                output_buffer.copy_from_slice(input_buffer);
                return;
            }
        };
        
        // Prepare node states
        let mut node_states = HashMap::new();
        for &node_id in &processing_order {
            node_states.insert(node_id, NodeState::default());
        }
        
        // Process nodes in topological order
        for &node_id in &processing_order {
            // First, collect inputs from connections (requires immutable borrow)
            {
                let node_state = node_states.get_mut(&node_id).unwrap();
                
                // Clear input buffer
                node_state.input_buffer.fill(0.0);
            }
            
            // Collect inputs from connections (separate scope for borrow checker)
            // First collect all source buffers we need
            let mut input_buffers = Vec::new();
            for connection in &self.connections {
                if connection.to_node == node_id && connection.to_port == "audio_in" {
                    if let Some(source_state) = node_states.get(&connection.from_node) {
                        input_buffers.push(source_state.output_buffer.clone());
                    }
                }
            }
            
            // Now mix all inputs into the node's input buffer
            if let Some(node_state) = node_states.get_mut(&node_id) {
                for input_buffer_data in input_buffers {
                    for (i, &sample) in input_buffer_data.iter().enumerate() {
                        if i < node_state.input_buffer.len() {
                            node_state.input_buffer[i] += sample;
                        }
                    }
                }
            }
            
            // Process the node
            if let Some(node) = self.nodes.get_mut(&node_id) {
                let node_state = node_states.get_mut(&node_id).unwrap();
                
                // Add external input if this is an input node
                if node.node_type == NodeType::Input {
                    let input_len = input_buffer.len().min(node_state.output_buffer.len());
                    for i in 0..input_len {
                        node_state.output_buffer[i] = input_buffer[i];
                    }
                } else if let Some(ref mut dsp_module) = node.dsp_module {
                    // Process through DSP module
                    dsp_module.process(&node_state.input_buffer, &mut node_state.output_buffer);
                } else {
                    // For non-DSP nodes (mixers, etc.), just pass through
                    node_state.output_buffer.copy_from_slice(&node_state.input_buffer);
                }
            }
            
            // Apply node-specific processing
            let node_state = node_states.get_mut(&node_id).unwrap();
            self.apply_node_processing(node_id, &mut node_state.output_buffer);
        }
        
        // Get final output from output node or last node
        let final_output_node = self.output_node.unwrap_or_else(|| {
            processing_order.last().copied().unwrap_or(0)
        });
        
        if let Some(node_state) = node_states.get(&final_output_node) {
            let mono_len = node_state.output_buffer.len();
            let out_len = output_buffer.len();
            // Detect stereo interleaved if output buffer appears to be 2x mono size
            // Fallback to mono copy otherwise.
            let stereo_frames = if out_len >= mono_len * 2 { mono_len } else { 0 };

            if stereo_frames > 0 {
                // Equal-power pan and gain ramp smoothing across the buffer
                let pan_start = self.master_pan_smooth;
                let pan_target = self.master_pan.clamp(-1.0, 1.0) as f32;
                let gain_start = self.master_gain_smooth;
                let gain_target = self.master_volume;
                for frame in 0..stereo_frames {
                    let frac = if stereo_frames > 0 { frame as f32 / stereo_frames as f32 } else { 1.0 };
                    let pan = pan_start + (pan_target - pan_start) * frac;
                    let gain = gain_start + (gain_target - gain_start) * frac;
                    let theta = (pan + 1.0) * std::f32::consts::FRAC_PI_4; // equal-power
                    let (lg, rg) = (theta.cos(), theta.sin());
                    let s = node_state.output_buffer[frame] * gain;
                    let base = frame * 2;
                    if base + 1 < out_len {
                        output_buffer[base] = s * lg;      // Left
                        output_buffer[base + 1] = s * rg;  // Right
                    }
                }
                // Commit smoothed states
                self.master_pan_smooth = pan_target;
                self.master_gain_smooth = gain_target;
            } else {
                let output_len = out_len.min(mono_len);
                let gain_start = self.master_gain_smooth;
                let gain_target = self.master_volume;
                for i in 0..output_len {
                    let frac = if output_len > 0 { i as f32 / output_len as f32 } else { 1.0 };
                    let gain = gain_start + (gain_target - gain_start) * frac;
                    output_buffer[i] = node_state.output_buffer[i] * gain;
                }
                self.master_gain_smooth = gain_target;
            }
        } else {
            output_buffer.copy_from_slice(input_buffer);
        }
        
        // Update timing
        self.audio_flow.current_time += output_buffer.len() as f64 / self.audio_flow.sample_rate as f64;
    }

    /// Set a parameter on a node (updates cached value and underlying DSP module)
    pub fn set_node_parameter(&mut self, node_id: usize, param: &str, value: f32) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.parameters.insert(param.to_string(), value);
            if let Some(ref mut dsp) = node.dsp_module {
                dsp.set_parameter(param, value);
            }
        }
    }
    
    /// Apply node-specific processing (like applying track volumes)
    fn apply_node_processing(&mut self, node_id: usize, buffer: &mut [f32]) {
        // This is where we would apply track volumes, automation, etc.
        // For now, just placeholder
        if let Some(node) = self.nodes.get(&node_id) {
            if matches!(node.node_type, NodeType::Input) {
                // Apply track volume if this is an input node
                let tracks_len = self.track_volumes.len();
                if tracks_len > 0 {
                    let track_idx = node_id % tracks_len;
                    let gain_start = self.track_gain_smooth[track_idx];
                    let gain_target = self.track_volumes[track_idx];
                    let n = buffer.len();
                    for (i, sample) in buffer.iter_mut().enumerate() {
                        let frac = if n > 0 { i as f32 / n as f32 } else { 1.0 };
                        let gain = gain_start + (gain_target - gain_start) * frac;
                        *sample *= gain;
                    }
                    // Commit smoothed gain for this track
                    self.track_gain_smooth[track_idx] = gain_target;
                }
            }
        }
    }
    
    /// Get processing order using topological sort
    fn get_processing_order(&self) -> Result<Vec<usize>, String> {
        let mut in_degree = HashMap::new();
        let mut adjacency = HashMap::new();
        
        // Initialize in-degrees
        for (&node_id, _) in &self.nodes {
            in_degree.insert(node_id, 0);
            adjacency.insert(node_id, Vec::new());
        }
        
        // Build adjacency list and calculate in-degrees
        for connection in &self.connections {
            adjacency.entry(connection.from_node)
                .or_insert_with(Vec::new)
                .push(connection.to_node);
            
            *in_degree.entry(connection.to_node).or_insert(0) += 1;
        }
        
        // Topological sort using Kahn's algorithm
        let mut queue: VecDeque<usize> = in_degree.iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(&node_id, _)| node_id)
            .collect();
            
        let mut result = Vec::new();
        
        while let Some(node_id) = queue.pop_front() {
            result.push(node_id);
            
            if let Some(neighbors) = adjacency.get(&node_id) {
                for &neighbor_id in neighbors {
                    if let Some(degree) = in_degree.get_mut(&neighbor_id) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor_id);
                        }
                    }
                }
            }
        }
        
        // Check for cycles
        if result.len() != self.nodes.len() {
            return Err("Cycle detected in audio graph".to_string());
        }
        
        Ok(result)
    }
    
    /// Set master volume
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.max(0.0).min(1.0);
    }

    /// Set master pan (-1.0 left .. 1.0 right)
    pub fn set_master_pan(&mut self, pan: f32) {
        self.master_pan = pan.max(-1.0).min(1.0);
    }

    /// Get master volume (current target)
    pub fn get_master_volume(&self) -> f32 {
        self.master_volume
    }

    /// Get master pan (current target)
    pub fn get_master_pan(&self) -> f32 {
        self.master_pan
    }
    
    /// Set track volume
    pub fn set_track_volume(&mut self, track: usize, volume: f32) {
        let vol = volume.max(0.0).min(1.0);
        if track < self.track_volumes.len() {
            self.track_volumes[track] = vol;
        } else {
            let new_len = track + 1;
            let _current_len = self.track_volumes.len();
            self.track_volumes.resize(new_len, 0.8);
            self.track_gain_smooth.resize(new_len, 0.8);
            // Initialize both target and smooth to requested volume for new track
            self.track_volumes[track] = vol;
            self.track_gain_smooth[track] = vol;
        }
    }

    /// Set track pan (-1.0 left .. 1.0 right)
    pub fn set_track_pan(&mut self, track: usize, pan: f32) {
        if track < self.track_pans.len() {
            self.track_pans[track] = pan.max(-1.0).min(1.0);
        }
    }

    /// Set return bus volume
    pub fn set_return_volume(&mut self, bus: usize, volume: f32) {
        if bus < self.return_volumes.len() {
            self.return_volumes[bus] = volume.max(0.0).min(1.0);
        }
    }

    /// Set return bus pan (-1.0 left .. 1.0 right)
    pub fn set_return_pan(&mut self, bus: usize, pan: f32) {
        if bus < self.return_pans.len() {
            self.return_pans[bus] = pan.max(-1.0).min(1.0);
        }
    }

    /// Get track volumes
    pub fn get_track_volumes(&self) -> Vec<f32> {
        self.track_volumes.clone()
    }

    /// Get track pans
    pub fn get_track_pans(&self) -> Vec<f32> {
        self.track_pans.clone()
    }

    /// Get return bus volumes
    pub fn get_return_volumes(&self) -> Vec<f32> {
        self.return_volumes.clone()
    }

    /// Get return bus pans
    pub fn get_return_pans(&self) -> Vec<f32> {
        self.return_pans.clone()
    }
    
    /// Get master peak level (simplified)
    pub fn get_master_peak(&self) -> f32 {
        // In a real implementation, this would track actual peak levels
        -6.0 // Return a placeholder level
    }
    
    /// Get track peak levels (simplified)
    pub fn get_track_peaks(&self) -> Vec<f32> {
        // In a real implementation, this would track actual peak levels per track
        vec![-12.0; self.track_volumes.len()]
    }

    /// Get return bus peak levels (simplified, based on volume)
    pub fn get_return_peaks(&self) -> Vec<f32> {
        // Placeholder: derive peaks from return volumes
        self.return_volumes
            .iter()
            .map(|v| v * 20.0 - 60.0)
            .collect()
    }
    
    /// Get spectrum data (simplified)
    pub fn get_spectrum_data(&self) -> Vec<f32> {
        // In a real implementation, this would perform FFT analysis
        let mut spectrum = Vec::with_capacity(256);
        for i in 0..256 {
            let freq = i as f32 * self.audio_flow.sample_rate as f32 / 512.0;
            let magnitude = if freq < 1000.0 {
                1.0 - (freq / 1000.0)
            } else {
                0.1
            };
            spectrum.push(magnitude * 20.0 * (i as f32 * 0.1).sin().abs());
        }
        spectrum
    }
    
    /// Remove a node
    pub fn remove_node(&mut self, node_id: usize) {
        // Remove connections involving this node
        self.connections.retain(|conn| conn.from_node != node_id && conn.to_node != node_id);
        
        // Remove the node
        self.nodes.remove(&node_id);
        
        // Update output node reference
        if self.output_node == Some(node_id) {
            self.output_node = None;
        }
        if self.input_node == Some(node_id) {
            self.input_node = None;
        }
    }
    
    /// Get node information
    #[allow(dead_code)]
    pub(crate) fn get_node_info(&self, node_id: usize) -> Option<&GraphNode> {
        self.nodes.get(&node_id)
    }
    
    /// Get all connections
    pub fn get_connections(&self) -> &[NodeConnection] {
        &self.connections
    }
    
    /// Clear the entire graph
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.connections.clear();
        self.output_node = None;
        self.input_node = None;
    }
}

impl Default for NodeGraph {
    fn default() -> Self {
        Self::new()
    }
}