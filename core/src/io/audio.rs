use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use crossbeam_channel::Sender;
use crate::engine::{CoreEngine, EngineCommand};

/// Callback signature run on the audio thread. Returns processed audio into
/// `output` (mono block). The engine is owned by the caller; this type just
/// holds the `cpal` stream handle.
pub struct AudioDeviceManager {
    _stream: cpal::Stream,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioDeviceManager {
    /// Open the default output device and run `process` on the audio thread.
    ///
    /// `process` is invoked with a mutable engine reference so the audio graph
    /// can be advanced in the real-time callback. It should write `frames`
    /// samples into the output slice (mono). On failure returns an error
    /// string (e.g. when no audio device is available).
    pub fn init_cpal<F>(process: F) -> Result<Self, String>
    where
        F: FnMut(&mut [f32]) + Send + 'static,
    {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "no default output device".to_string())?;
        let config = device
            .default_output_config()
            .map_err(|e| format!("failed to get output config: {e}"))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        let mut processed = process;
        let err_fn = |err| eprintln!("audio stream error: {err}");
        // Persistent scratch buffer reused across callbacks to avoid
        // per-frame allocations on the real-time audio thread.
        let mut mono_scratch: Vec<f32> = Vec::new();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                device.build_output_stream(
                    &config.into(),
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        let frames = if channels > 0 { data.len() / channels as usize } else { 0 };
                        if frames == 0 { return; }
                        mono_scratch.resize(frames, 0.0);
                        processed(&mut mono_scratch);
                        // Copy the mono block to all channels (interleaved).
                        for (i, frame) in data.chunks_mut(channels as usize).enumerate() {
                            let v = mono_scratch.get(i).copied().unwrap_or(0.0);
                            for ch in frame.iter_mut() {
                                *ch = v;
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                device.build_output_stream(
                    &config.into(),
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        let frames = if channels > 0 { data.len() / channels as usize } else { 0 };
                        if frames == 0 { return; }
                        mono_scratch.resize(frames, 0.0);
                        processed(&mut mono_scratch);
                        for (i, frame) in data.chunks_mut(channels as usize).enumerate() {
                            let v = mono_scratch.get(i).copied().unwrap_or(0.0);
                            let v = (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                            for ch in frame.iter_mut() {
                                *ch = v;
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                device.build_output_stream(
                    &config.into(),
                    move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                        let frames = if channels > 0 { data.len() / channels as usize } else { 0 };
                        if frames == 0 { return; }
                        mono_scratch.resize(frames, 0.0);
                        processed(&mut mono_scratch);
                        for (i, frame) in data.chunks_mut(channels as usize).enumerate() {
                            let v = mono_scratch.get(i).copied().unwrap_or(0.0);
                            let v = ((v.clamp(-1.0, 1.0) * 0.5 + 0.5) * u16::MAX as f32) as u16;
                            for ch in frame.iter_mut() {
                                *ch = v;
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            other => {
                return Err(format!("unsupported sample format: {other:?}"));
            }
        }
        .map_err(|e| format!("failed to build output stream: {e}"))?;

        stream.play().map_err(|e| format!("failed to start stream: {e}"))?;

        Ok(Self {
            _stream: stream,
            sample_rate,
            channels,
        })
    }
}

/// Convenience wrapper that owns a `CoreEngine` and runs it from a cpal
/// callback. The engine is shared via `Arc<Mutex<CoreEngine>>` so the UI/control
/// thread can send commands and read state.
pub struct CpalEngineHost {
    pub engine: Arc<Mutex<CoreEngine>>,
    pub cmd_tx: Sender<EngineCommand>,
    pub device_manager: Option<AudioDeviceManager>,
}

impl CpalEngineHost {
    pub fn new(config: crate::config::EngineConfig, sample_rate: f32) -> Self {
        let (engine, cmd_tx) = CoreEngine::new(config, sample_rate);
        Self {
            engine: Arc::new(Mutex::new(engine)),
            cmd_tx,
            device_manager: None,
        }
    }

    /// Clone of the command sender, for pushing `EngineCommand`s from the UI,
    /// MIDI, or network threads.
    pub fn command_sender(&self) -> Sender<EngineCommand> {
        self.cmd_tx.clone()
    }

    /// Shared handle to the engine (for reading status/snapshots from other threads).
    pub fn engine(&self) -> Arc<Mutex<CoreEngine>> {
        Arc::clone(&self.engine)
    }

    pub fn start(&mut self) -> Result<(), String> {
        let engine = Arc::clone(&self.engine);
        let dm = AudioDeviceManager::init_cpal(move |output| {
            let mut eng = engine.lock().unwrap();
            eng.process_block(output);
        })?;
        self.device_manager = Some(dm);
        Ok(())
    }
}
