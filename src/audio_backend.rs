//! Real-time Audio Processing Backend
//!
//! This module provides the core audio processing engine for HexoDSP DAW,
//! handling real-time audio I/O, processing, and synchronization.

use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::collections::VecDeque;
use crate::node_graph::{NeuroNodeGraph, NeuroNodeProcessor};
use crate::audio_nodes::AudioBuffer;

/// Audio Backend Configuration
#[derive(Debug, Clone)]
pub struct AudioBackendConfig {
    pub sample_rate: f32,
    pub buffer_size: usize,
    pub num_input_channels: usize,
    pub num_output_channels: usize,
    pub enable_realtime: bool,
}

impl Default for AudioBackendConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100.0,
            buffer_size: 512,
            num_input_channels: 2,
            num_output_channels: 2,
            enable_realtime: true,
        }
    }
}

/// Real-time Audio Processor
pub struct RealtimeAudioProcessor {
    config: AudioBackendConfig,
    node_graph: Arc<Mutex<NeuroNodeGraph>>,
    input_buffer: VecDeque<f32>,
    output_buffer: VecDeque<f32>,
    is_running: Arc<AtomicBool>,
    current_time: Arc<Mutex<f64>>,
}

impl RealtimeAudioProcessor {
    pub fn new(config: AudioBackendConfig, node_graph: Arc<Mutex<NeuroNodeGraph>>) -> Self {
        let config_clone = config.clone();
        Self {
            config,
            node_graph,
            input_buffer: VecDeque::with_capacity(config_clone.buffer_size * config_clone.num_input_channels),
            output_buffer: VecDeque::with_capacity(config_clone.buffer_size * config_clone.num_output_channels),
            is_running: Arc::new(AtomicBool::new(false)),
            current_time: Arc::new(Mutex::new(0.0)),
        }
    }

    /// Start real-time audio processing
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.is_running.store(true, Ordering::SeqCst);
        println!("Real-time audio processor started");
        Ok(())
    }

    /// Stop real-time audio processing
    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.is_running.store(false, Ordering::SeqCst);
        println!("Real-time audio processor stopped");
        Ok(())
    }

    /// Process a block of audio samples
    pub fn process_block(&mut self, input: &[f32], output: &mut [f32]) -> Result<(), Box<dyn std::error::Error>> {
        if !self.is_running.load(Ordering::SeqCst) {
            // Pass through input to output when not processing
            output.copy_from_slice(input);
            return Ok(());
        }

        // Convert interleaved audio to our buffer format
        let input_buffer = self.interleaved_to_buffer(input);

        // Process through node graph
        let processed_buffer = self.process_through_graph(input_buffer)?;

        // Convert back to interleaved format
        self.buffer_to_interleaved(&processed_buffer, output);

        // Update timing
        let mut time = self.current_time.lock().unwrap();
        *time += output.len() as f64 / (self.config.sample_rate as f64 * self.config.num_output_channels as f64);

        Ok(())
    }

    /// Process audio through the node graph
    fn process_through_graph(&self, input: AudioBuffer) -> Result<AudioBuffer, Box<dyn std::error::Error>> {
        let graph = self.node_graph.lock().unwrap();

        // Get the processing order from the graph
        let processing_order = graph.evaluation_order();

        let mut current_buffer = input;

        // Process through each node in order
        for _node_id in processing_order {
            // In a real implementation, this would call the node's process method
            // For now, we'll just pass the buffer through
            current_buffer = self.process_node_placeholder(current_buffer)?;
        }

        Ok(current_buffer)
    }

    /// Process a single node (placeholder implementation)
    fn process_node_placeholder(&self, input: AudioBuffer) -> Result<AudioBuffer, Box<dyn std::error::Error>> {
        // Placeholder - in a real implementation, this would call the node's process method
        // For now, just return the input unchanged
        Ok(input)
    }

    /// Convert interleaved audio samples to AudioBuffer
    fn interleaved_to_buffer(&self, interleaved: &[f32]) -> AudioBuffer {
        let mut buffer = AudioBuffer::new(interleaved.len() / self.config.num_input_channels);

        for (i, &sample) in interleaved.iter().enumerate() {
            let frame_index = i / self.config.num_input_channels;
            let channel = i % self.config.num_input_channels;

            if channel == 0 {
                buffer.set(frame_index, crate::audio_nodes::StereoFrame::new(sample, sample));
            }
        }

        buffer
    }

    /// Convert AudioBuffer to interleaved audio samples
    fn buffer_to_interleaved(&self, buffer: &AudioBuffer, interleaved: &mut [f32]) {
        for (i, frame) in buffer.samples.iter().enumerate() {
            let base_index = i * self.config.num_output_channels;

            if base_index < interleaved.len() {
                interleaved[base_index] = frame.left;
            }

            if base_index + 1 < interleaved.len() {
                interleaved[base_index + 1] = frame.right;
            }
        }
    }

    /// Get current processing latency in samples
    pub fn get_latency_samples(&self) -> usize {
        self.config.buffer_size
    }

    /// Get current processing latency in milliseconds
    pub fn get_latency_ms(&self) -> f32 {
        (self.get_latency_samples() as f32 / self.config.sample_rate) * 1000.0
    }

    /// Check if processor is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Get current processing time
    pub fn get_current_time(&self) -> f64 {
        *self.current_time.lock().unwrap()
    }
}

/// Audio Device Manager
pub struct AudioDeviceManager {
    available_devices: Vec<AudioDevice>,
    current_input_device: Option<usize>,
    current_output_device: Option<usize>,
}

impl AudioDeviceManager {
    pub fn new() -> Self {
        Self {
            available_devices: Vec::new(),
            current_input_device: None,
            current_output_device: None,
        }
    }

    /// Enumerate available audio devices
    pub fn enumerate_devices(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // In a real implementation, this would query the system for audio devices
        // For now, we'll add some placeholder devices

        self.available_devices = vec![
            AudioDevice {
                id: 0,
                name: "Default System Audio".to_string(),
                input_channels: 2,
                output_channels: 2,
                sample_rates: vec![44100.0, 48000.0, 96000.0],
                is_default: true,
            },
            AudioDevice {
                id: 1,
                name: "High-End Audio Interface".to_string(),
                input_channels: 8,
                output_channels: 8,
                sample_rates: vec![44100.0, 48000.0, 88200.0, 96000.0, 192000.0],
                is_default: false,
            },
        ];

        println!("Found {} audio devices", self.available_devices.len());
        Ok(())
    }

    /// Get list of available devices
    pub fn get_devices(&self) -> &[AudioDevice] {
        &self.available_devices
    }

    /// Set input device
    pub fn set_input_device(&mut self, device_id: usize) -> Result<(), Box<dyn std::error::Error>> {
        if device_id >= self.available_devices.len() {
            return Err("Invalid device ID".into());
        }

        self.current_input_device = Some(device_id);
        println!("Set input device to: {}", self.available_devices[device_id].name);
        Ok(())
    }

    /// Set output device
    pub fn set_output_device(&mut self, device_id: usize) -> Result<(), Box<dyn std::error::Error>> {
        if device_id >= self.available_devices.len() {
            return Err("Invalid device ID".into());
        }

        self.current_output_device = Some(device_id);
        println!("Set output device to: {}", self.available_devices[device_id].name);
        Ok(())
    }

    /// Get current input device
    pub fn get_current_input_device(&self) -> Option<&AudioDevice> {
        self.current_input_device.and_then(|id| self.available_devices.get(id))
    }

    /// Get current output device
    pub fn get_current_output_device(&self) -> Option<&AudioDevice> {
        self.current_output_device.and_then(|id| self.available_devices.get(id))
    }
}

/// Audio Device Information
#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub id: usize,
    pub name: String,
    pub input_channels: usize,
    pub output_channels: usize,
    pub sample_rates: Vec<f32>,
    pub is_default: bool,
}

/// Audio Stream Manager
pub struct AudioStreamManager {
    processor: Arc<Mutex<RealtimeAudioProcessor>>,
    device_manager: AudioDeviceManager,
    stream_active: Arc<AtomicBool>,
}

impl AudioStreamManager {
    pub fn new(processor: Arc<Mutex<RealtimeAudioProcessor>>) -> Self {
        Self {
            processor,
            device_manager: AudioDeviceManager::new(),
            stream_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Initialize audio system
    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Enumerate devices
        self.device_manager.enumerate_devices()?;

        // Set default devices
        let devices = self.device_manager.get_devices().to_vec();
        if let Some(default_device) = devices.iter().find(|d| d.is_default) {
            self.device_manager.set_input_device(default_device.id)?;
            self.device_manager.set_output_device(default_device.id)?;
        }

        println!("Audio system initialized");
        Ok(())
    }

    /// Start audio stream
    pub async fn start_stream(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.stream_active.load(Ordering::SeqCst) {
            return Err("Stream already active".into());
        }

        // Start the processor
        self.processor.lock().unwrap().start()?;
        self.stream_active.store(true, Ordering::SeqCst);

        println!("Audio stream started");
        Ok(())
    }

    /// Stop audio stream
    pub async fn stop_stream(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.stream_active.load(Ordering::SeqCst) {
            return Err("Stream not active".into());
        }

        // Stop the processor
        self.processor.lock().unwrap().stop()?;
        self.stream_active.store(false, Ordering::SeqCst);

        println!("Audio stream stopped");
        Ok(())
    }

    /// Check if stream is active
    pub fn is_stream_active(&self) -> bool {
        self.stream_active.load(Ordering::SeqCst)
    }

    /// Get device manager reference
    pub fn device_manager(&mut self) -> &mut AudioDeviceManager {
        &mut self.device_manager
    }

    /// Get processor reference
    pub fn processor(&self) -> &Arc<Mutex<RealtimeAudioProcessor>> {
        &self.processor
    }
}

/// Performance Monitor for Audio Processing
pub struct AudioPerformanceMonitor {
    xruns: Arc<Mutex<usize>>,
    average_latency: Arc<Mutex<f32>>,
    cpu_usage: Arc<Mutex<f32>>,
}

impl AudioPerformanceMonitor {
    pub fn new() -> Self {
        Self {
            xruns: Arc::new(Mutex::new(0)),
            average_latency: Arc::new(Mutex::new(0.0)),
            cpu_usage: Arc::new(Mutex::new(0.0)),
        }
    }

    /// Record an XRUN (buffer underrun/overrun)
    pub fn record_xrun(&self) {
        let mut xruns = self.xruns.lock().unwrap();
        *xruns += 1;
    }

    /// Update latency measurement
    pub fn update_latency(&self, latency_ms: f32) {
        let mut avg_latency = self.average_latency.lock().unwrap();
        *avg_latency = (*avg_latency + latency_ms) / 2.0; // Simple moving average
    }

    /// Update CPU usage
    pub fn update_cpu_usage(&self, cpu_percent: f32) {
        let mut cpu = self.cpu_usage.lock().unwrap();
        *cpu = cpu_percent;
    }

    /// Get performance stats
    pub fn get_stats(&self) -> AudioPerformanceStats {
        AudioPerformanceStats {
            xruns: *self.xruns.lock().unwrap(),
            average_latency_ms: *self.average_latency.lock().unwrap(),
            cpu_usage_percent: *self.cpu_usage.lock().unwrap(),
        }
    }
}

/// Audio Performance Statistics
#[derive(Debug, Clone)]
pub struct AudioPerformanceStats {
    pub xruns: usize,
    pub average_latency_ms: f32,
    pub cpu_usage_percent: f32,
}