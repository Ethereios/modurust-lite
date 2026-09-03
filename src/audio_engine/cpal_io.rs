/// Real-time audio I/O using CPAL (Cross-Platform Audio Library)
/// 
/// This module handles low-latency audio device communication,
/// supporting ASIO, WASAPI, CoreAudio, and other native APIs.

use cpal::{Device, Host, Stream, StreamConfig, SampleFormat, InputCallbackInfo, OutputCallbackInfo};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use crate::event_queue::{EventQueue, EventType, AudioThreadEventProcessor};

/// Audio configuration structure
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub channels: u16,
    pub device_name: String,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            buffer_size: 256, // Small buffer for low latency
            channels: 2,      // Stereo
            device_name: "Default".to_string(),
        }
    }
}

/// Audio device information
#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub name: String,
    pub is_input: bool,
    pub is_output: bool,
    pub max_channels: u16,
    pub default_sample_rate: u32,
}

/// Audio I/O system for real-time processing
pub struct AudioIO {
    host: Host,
    input_device: Option<Device>,
    output_device: Option<Device>,
    config: AudioConfig,
    input_stream: Option<Stream>,
    output_stream: Option<Stream>,
    running: Arc<AtomicBool>,
    buffer_size: u32,
    sample_rate: u32,
    event_queue: Arc<EventQueue>,
    // Simple test tone controls (until full graph processing is wired)
    tone_freq: Arc<Mutex<f32>>, // Hz
    tone_amp: Arc<Mutex<f32>>,  // 0.0..=1.0
    tone_amp_smooth: Arc<Mutex<f32>>, // smoothed amplitude across buffers
    tone_pan: Arc<Mutex<f32>>,  // -1.0..=+1.0 (target)
    tone_pan_smooth: Arc<Mutex<f32>>, // smoothed value across buffers
    // Transport-gated playback flag: when false, output is silent regardless of tone params
    playback_enabled: Arc<AtomicBool>,
}

impl AudioIO {
    /// Create a new audio I/O system
    pub fn new(event_queue: Arc<EventQueue>) -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let config = AudioConfig::default();
        
        println!("🎵 Audio Host: {:?}", host.id());
        
        // Get default devices
        let input_device = host.default_input_device()
            .map(|dev| {
                println!("🎤 Input Device: {}", dev.name().unwrap_or_else(|_| "Unknown".to_string()));
                dev
            });
            
        let output_device = host.default_output_device()
            .map(|dev| {
                println!("🔊 Output Device: {}", dev.name().unwrap_or_else(|_| "Unknown".to_string()));
                dev
            });

        let buffer_size = config.buffer_size;
        let sample_rate = config.sample_rate;
        
        Ok(Self {
            host,
            input_device,
            output_device,
            config,
            input_stream: None,
            output_stream: None,
            running: Arc::new(AtomicBool::new(false)),
            buffer_size,
            sample_rate,
            event_queue,
            tone_freq: Arc::new(Mutex::new(440.0)),
            // Default to silence to avoid unwanted constant tone
            tone_amp: Arc::new(Mutex::new(0.0)),
            tone_amp_smooth: Arc::new(Mutex::new(0.0)),
            tone_pan: Arc::new(Mutex::new(0.0)),
            tone_pan_smooth: Arc::new(Mutex::new(0.0)),
            playback_enabled: Arc::new(AtomicBool::new(false)),
        })
    }
    
    /// Get list of available audio devices
    pub fn list_devices(&self) -> Result<Vec<AudioDevice>, Box<dyn std::error::Error>> {
        let mut devices = Vec::new();
        
        // Input devices
        for device in self.host.input_devices()? {
            let name = device.name()?.to_string();
            let config = device.default_input_config()?;
            
            devices.push(AudioDevice {
                name: name.clone(),
                is_input: true,
                is_output: false,
                max_channels: config.channels(),
                default_sample_rate: config.sample_rate().0,
            });
            
            println!("🎤 Input Device: {} ({} channels, {} Hz)", 
                name, config.channels(), config.sample_rate().0);
        }
        
        // Output devices  
        for device in self.host.output_devices()? {
            let name = device.name()?.to_string();
            let config = device.default_output_config()?;
            
            devices.push(AudioDevice {
                name: name.clone(),
                is_input: false,
                is_output: true,
                max_channels: config.channels(),
                default_sample_rate: config.sample_rate().0,
            });
            
            println!("🔊 Output Device: {} ({} channels, {} Hz)", 
                name, config.channels(), config.sample_rate().0);
        }
        
        Ok(devices)
    }
    
    /// Set audio configuration
    pub fn set_config(&mut self, config: AudioConfig) -> Result<(), Box<dyn std::error::Error>> {
        self.buffer_size = config.buffer_size;
        self.sample_rate = config.sample_rate;
        self.config = config;
        Ok(())
    }
    
    /// Get current configuration
    pub fn config(&self) -> &AudioConfig {
        &self.config
    }
    
    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    
    /// Get buffer size
    pub fn buffer_size(&self) -> u32 {
        self.buffer_size
    }
    
    /// Start audio streams with callback
    pub fn start<F>(&mut self, mut input_callback: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(&[f32], &cpal::InputCallbackInfo) + Send + 'static,
    {
        if self.running.load(Ordering::SeqCst) {
            println!("⚠️ Audio streams already running");
            return Ok(());
        }

        println!("🎵 Starting audio streams...");
        
        // Set up output stream
        if let Some(device) = &self.output_device {
            let config = self.create_output_config()?;
            let running = Arc::clone(&self.running);
            
            let tone_freq = Arc::clone(&self.tone_freq);
            let tone_amp = Arc::clone(&self.tone_amp);
            let tone_amp_smooth = Arc::clone(&self.tone_amp_smooth);
            let tone_pan = Arc::clone(&self.tone_pan);
            let tone_pan_smooth = Arc::clone(&self.tone_pan_smooth);
            let playback_enabled = Arc::clone(&self.playback_enabled);
            let channels = config.channels as usize;
            let event_queue = Arc::clone(&self.event_queue);
            let output_callback = move |data: &mut [f32], _: &OutputCallbackInfo| {
                if !running.load(Ordering::SeqCst) {
                    // Fill with silence if not running
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                    return;
                }

                // Gate playback by transport/Play state
                if !playback_enabled.load(Ordering::SeqCst) {
                    for sample in data.iter_mut() { *sample = 0.0; }
                    return;
                }
                
                // Generate test tone (sine wave)
                let sample_rate = 44100.0;
                let freq = *tone_freq.lock().unwrap();
                // Target amplitude and starting smoothed amplitude
                let amp_target = (*tone_amp.lock().unwrap()).clamp(0.0, 1.0);
                let amp_start = *tone_amp_smooth.lock().unwrap();
                // If amplitude is zero, output silence for the whole buffer
                if amp_target <= 0.000_001 && amp_start <= 0.000_001 {
                    for sample in data.iter_mut() { *sample = 0.0; }
                    return;
                }
                let time = std::time::SystemTime::now();
                let elapsed = time.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                let seconds = elapsed.as_secs_f64();
                
                // Create an event processor for this buffer
                let mut event_processor = AudioThreadEventProcessor::new(Arc::clone(&event_queue));
                event_processor.set_current_sample(seconds as u64 * sample_rate as u64);

                // Process events for this buffer
                event_processor.process_buffer(data.len(), |event, sample_offset| {
                    match event {
                        EventType::ParamChange { node_id, param_id, value, curve_type } => {
                            // TODO: Apply parameter change to the audio engine state
                            // For now, let's just print it
                            println!("ParamChange: node_id={}, param_id={}, value={}, curve_type={:?}, sample_offset={}",
                                node_id, param_id, value, curve_type, sample_offset);
                        },
                        EventType::MidiEvent { status, data1, data2, channel, target_node_id } => {
                            // TODO: Process MIDI event
                            println!("MidiEvent: status={}, data1={}, data2={}, channel={}, target_node_id={}",
                                status, data1, data2, channel, target_node_id);
                        },
                        _ => {},
                    }
                });
                
                // Smooth pan over this buffer to avoid zipper noise
                let pan_target = (*tone_pan.lock().unwrap()).clamp(-1.0, 1.0);
                let pan_start = *tone_pan_smooth.lock().unwrap();

                if channels >= 2 {
                    let frames = data.len() / channels;
                    for frame in 0..frames {
                        // Linear ramp from start -> target across frames
                        let frac = if frames > 0 { frame as f32 / frames as f32 } else { 1.0 };
                        let pan = pan_start + (pan_target - pan_start) * frac;
                        let amp = amp_start + (amp_target - amp_start) * frac;
                        let theta = (pan + 1.0) as f32 * std::f32::consts::FRAC_PI_4; // equal-power
                        let (lg, rg) = (theta.cos(), theta.sin());
                        let t = seconds + (frame as f64 / sample_rate);
                        let s_base = (2.0 * std::f64::consts::PI * freq as f64 * t).sin() as f32;
                        let s = s_base * amp;
                        let base = frame * channels;
                        data[base] = s * lg;         // Left
                        data[base + 1] = s * rg;     // Right
                        for ch in 2..channels {       // Additional channels copy
                            data[base + ch] = s;
                        }
                    }
                    // Commit smoothed pan to target for continuity across buffers
                    if let Ok(mut p) = tone_pan_smooth.lock() { *p = pan_target; }
                    if let Ok(mut a) = tone_amp_smooth.lock() { *a = amp_target; }
                } else {
                    let frames = data.len();
                    for (i, sample) in data.iter_mut().enumerate() {
                        let frac = if frames > 0 { i as f32 / frames as f32 } else { 1.0 };
                        let amp = amp_start + (amp_target - amp_start) * frac;
                        let t = seconds + (i as f64 / sample_rate);
                        let s_base = (2.0 * std::f64::consts::PI * freq as f64 * t).sin() as f32;
                        *sample = s_base * amp;
                    }
                    if let Ok(mut a) = tone_amp_smooth.lock() { *a = amp_target; }
                }
            };
            
            let stream = device.build_output_stream(
                &config,
                output_callback,
                |err| eprintln!("Output stream error: {}", err),
                None, // timeout
            )?;
            
            self.output_stream = Some(stream);
        }
        
        // Set up input stream
        if let Some(device) = &self.input_device {
            let config = self.create_input_config()?;
            let running = Arc::clone(&self.running);
            
            let input_callback = move |data: &[f32], info: &InputCallbackInfo| {
                if !running.load(Ordering::SeqCst) {
                    return;
                }
                
                // Call the user-provided callback
                input_callback(data, info);
            };
            
            let stream = device.build_input_stream(
                &config,
                input_callback,
                |err| eprintln!("Input stream error: {}", err),
                None, // timeout
            )?;
            
            self.input_stream = Some(stream);
        }
        
        // Start the streams
        if let Some(stream) = &self.output_stream {
            stream.play()?;
        }
        
        if let Some(stream) = &self.input_stream {
            stream.play()?;
        }
        
        self.running.store(true, Ordering::SeqCst);
        println!("✅ Audio streams started successfully");
        Ok(())
    }

    /// Start only output stream (no input monitoring to prevent feedback)
    pub fn start_output_only(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.running.load(Ordering::SeqCst) {
            println!("⚠️ Audio streams already running");
            return Ok(());
        }

        println!("🎵 Starting audio output stream only (input monitoring disabled)...");
        
        // Set up output stream only
        if let Some(device) = &self.output_device {
            let config = self.create_output_config()?;
            let running = Arc::clone(&self.running);
            
            let tone_freq = Arc::clone(&self.tone_freq);
            let tone_amp = Arc::clone(&self.tone_amp);
            let tone_amp_smooth = Arc::clone(&self.tone_amp_smooth);
            let tone_pan = Arc::clone(&self.tone_pan);
            let tone_pan_smooth = Arc::clone(&self.tone_pan_smooth);
            let playback_enabled = Arc::clone(&self.playback_enabled);
            let channels = config.channels as usize;
            let output_callback = move |data: &mut [f32], _: &OutputCallbackInfo| {
                if !running.load(Ordering::SeqCst) {
                    // Fill with silence if not running
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                    return;
                }

                // Gate playback by transport/Play state
                if !playback_enabled.load(Ordering::SeqCst) {
                    for sample in data.iter_mut() { *sample = 0.0; }
                    return;
                }
                
                // Generate test tone (sine wave)
                let sample_rate = 44100.0;
                let freq = *tone_freq.lock().unwrap();
                // Target amplitude and starting smoothed amplitude
                let amp_target = (*tone_amp.lock().unwrap()).clamp(0.0, 1.0);
                let amp_start = *tone_amp_smooth.lock().unwrap();
                // If amplitude is zero, output silence for the whole buffer
                if amp_target <= 0.000_001 && amp_start <= 0.000_001 {
                    for sample in data.iter_mut() { *sample = 0.0; }
                    return;
                }
                let time = std::time::SystemTime::now();
                let elapsed = time.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                let seconds = elapsed.as_secs_f64();
                
                // Smooth pan over this buffer to avoid zipper noise
                let pan_target = (*tone_pan.lock().unwrap()).clamp(-1.0, 1.0);
                let pan_start = *tone_pan_smooth.lock().unwrap();

                if channels >= 2 {
                    let frames = data.len() / channels;
                    for frame in 0..frames {
                        // Linear ramp from start -> target across frames
                        let frac = if frames > 0 { frame as f32 / frames as f32 } else { 1.0 };
                        let pan = pan_start + (pan_target - pan_start) * frac;
                        let amp = amp_start + (amp_target - amp_start) * frac;
                        let theta = (pan + 1.0) as f32 * std::f32::consts::FRAC_PI_4; // equal-power
                        let (lg, rg) = (theta.cos(), theta.sin());
                        let t = seconds + (frame as f64 / sample_rate);
                        let s_base = (2.0 * std::f64::consts::PI * freq as f64 * t).sin() as f32;
                        let s = s_base * amp;
                        let base = frame * channels;
                        data[base] = s * lg;         // Left
                        data[base + 1] = s * rg;     // Right
                        for ch in 2..channels {       // Additional channels copy
                            data[base + ch] = s;
                        }
                    }
                    // Commit smoothed pan to target for continuity across buffers
                    if let Ok(mut p) = tone_pan_smooth.lock() { *p = pan_target; }
                    if let Ok(mut a) = tone_amp_smooth.lock() { *a = amp_target; }
                } else {
                    let frames = data.len();
                    for (i, sample) in data.iter_mut().enumerate() {
                        let frac = if frames > 0 { i as f32 / frames as f32 } else { 1.0 };
                        let amp = amp_start + (amp_target - amp_start) * frac;
                        let t = seconds + (i as f64 / sample_rate);
                        let s_base = (2.0 * std::f64::consts::PI * freq as f64 * t).sin() as f32;
                        *sample = s_base * amp;
                    }
                    if let Ok(mut a) = tone_amp_smooth.lock() { *a = amp_target; }
                }
            };
            
            let stream = device.build_output_stream(
                &config,
                output_callback,
                |err| eprintln!("Output stream error: {}", err),
                None, // timeout
            )?;
            
            self.output_stream = Some(stream);
        }
        
        // Start only the output stream (no input stream to prevent feedback)
        if let Some(stream) = &self.output_stream {
            stream.play()?;
        }
        
        self.running.store(true, Ordering::SeqCst);
        println!("✅ Audio output stream started successfully (input monitoring disabled)");
        Ok(())
    }

    /// Set the test tone frequency (temporary audible control)
    pub fn set_tone_freq(&self, freq_hz: f32) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(mut f) = self.tone_freq.lock() {
            *f = freq_hz.max(1.0).min(20000.0);
        }
        Ok(())
    }

    /// Set the test tone amplitude (temporary audible control)
    pub fn set_tone_amp(&self, amp: f32) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(mut a) = self.tone_amp.lock() {
            *a = amp.clamp(0.0, 1.0);
        }
        Ok(())
    }

    /// Set the test tone pan (temporary audible control)
    pub fn set_tone_pan(&self, pan: f32) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(mut p) = self.tone_pan.lock() {
            *p = pan.clamp(-1.0, 1.0);
        }
        Ok(())
    }

    /// Enable/disable input monitoring at runtime.
    /// When disabled, drops the input stream to prevent feedback while keeping output active.
    /// Re-enabling input monitoring at runtime is not currently supported without a full restart.
    pub fn set_input_monitoring(&mut self, enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
        if !enabled {
            if self.input_stream.is_some() {
                // Drop the input stream to stop input monitoring
                self.input_stream = None;
                println!("🔇 Input monitoring disabled (input stream dropped)");
            } else {
                println!("ℹ️ Input monitoring already disabled");
            }
            return Ok(());
        }

        // Enabling input monitoring requires recreating the input stream with the processing callback,
        // which is managed by the engine start sequence. Log and no-op here.
        println!("⚠️ Enabling input monitoring at runtime is not supported; restart audio to enable.");
        Ok(())
    }
    
    /// Stop audio streams
    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        
        println!("🛑 Stopping audio streams...");
        
        self.running.store(false, Ordering::SeqCst);
        
        // Drop streams to stop them
        self.input_stream = None;
        self.output_stream = None;
        
        println!("✅ Audio streams stopped");
        Ok(())
    }
    
    /// Check if audio is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Enable or disable playback output (transport gating)
    pub fn set_playback_enabled(&self, enabled: bool) {
        self.playback_enabled.store(enabled, Ordering::SeqCst);
    }
    
    /// Create output stream configuration
    fn create_output_config(&self) -> Result<StreamConfig, Box<dyn std::error::Error>> {
        let device = self.output_device
            .as_ref()
            .ok_or("No output device available")?;
            
        let default_config = device.default_output_config()?;
        let sample_format = default_config.sample_format();
        
        // Convert to our format if needed
        if !matches!(sample_format, SampleFormat::F32) {
            println!("⚠️ Device uses {:?}, converting to F32", sample_format);
        }
        
        // Use device's default sample rate and channels to avoid StreamConfigNotSupported
        let sample_rate = default_config.sample_rate();
        let channels = default_config.channels();
        
        println!("🎛️ Using output config: {} Hz, {} channels", sample_rate.0, channels);
        
        Ok(StreamConfig {
            channels,
            sample_rate,
            buffer_size: cpal::BufferSize::Default, // Use device default buffer size
        })
    }
    
    /// Create input stream configuration
    fn create_input_config(&self) -> Result<StreamConfig, Box<dyn std::error::Error>> {
        let device = self.input_device
            .as_ref()
            .ok_or("No input device available")?;
            
        let default_config = device.default_input_config()?;
        let sample_format = default_config.sample_format();
        
        // Convert to our format if needed
        if !matches!(sample_format, SampleFormat::F32) {
            println!("⚠️ Device uses {:?}, converting to F32", sample_format);
        }
        
        // Use device's default sample rate and channels to avoid StreamConfigNotSupported
        let sample_rate = default_config.sample_rate();
        let channels = default_config.channels();
        
        println!("🎛️ Using input config: {} Hz, {} channels", sample_rate.0, channels);
        
        Ok(StreamConfig {
            channels,
            sample_rate,
            buffer_size: cpal::BufferSize::Default, // Use device default buffer size
        })
    }
    
    /// Get available sample rates for a device
    pub fn get_sample_rates(&self, device_name: &str) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
        let mut sample_rates = Vec::new();
        
        // Check input devices
        for device in self.host.input_devices()? {
            if device.name()? == device_name {
                let config = device.default_input_config()?;
                sample_rates.push(config.sample_rate().0);
                break;
            }
        }
        
        // Check output devices
        for device in self.host.output_devices()? {
            if device.name()? == device_name {
                let config = device.default_output_config()?;
                if !sample_rates.contains(&config.sample_rate().0) {
                    sample_rates.push(config.sample_rate().0);
                }
                break;
            }
        }
        
        // Add common sample rates as fallback
        if sample_rates.is_empty() {
            sample_rates = vec![44100, 48000, 88200, 96000, 192000];
        }
        
        Ok(sample_rates)
    }
    
    /// Set output device
    pub fn set_output_device(&mut self, device_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        for device in self.host.output_devices()? {
            if device.name()? == device_name {
                self.output_device = Some(device);
                self.config.device_name = device_name.to_string();
                return Ok(());
            }
        }
        Err(format!("Output device '{}' not found", device_name).into())
    }
    
    /// Set input device
    pub fn set_input_device(&mut self, device_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        for device in self.host.input_devices()? {
            if device.name()? == device_name {
                self.input_device = Some(device);
                return Ok(());
            }
        }
        Err(format!("Input device '{}' not found", device_name).into())
    }
}

impl Drop for AudioIO {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}