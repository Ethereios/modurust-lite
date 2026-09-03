#[macro_use]
extern crate vst;

use vst::buffer::AudioBuffer;
use vst::plugin::{Info, Plugin, PluginParameters};
use vst::util::AtomicFloat;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::net::UdpSocket;
use std::time::Duration;

use modurust_core::dsp::{FilterMode, Waveform};
use modurust_core::{CoreEngine, EngineConfig, EngineCommand, EngineMode, Param};

const PLUGIN_UNIQUE_ID: u32 = 0x6D6F6475; // "modu"

const UDP_CONTROL_PORT: u16 = 9000;
const UDP_STATUS_PORT: u16 = 9002;

plugin_main!(ModurustVst);

#[derive(Debug, Clone, Copy, PartialEq)]
enum VstParam {
    FilterCutoff = 0,
    FilterResonance = 1,
    FilterMode = 2,
    ReverbMix = 3,
    DelayTime = 4,
    DelayFeedback = 5,
    EnvAttack = 6,
    EnvRelease = 7,
    Waveform = 8,
    MasterGain = 9,
    PrimeBase = 10,
    PrimePhase = 11,
    NnSensitivity = 12,
    InstanceDensity = 13,
    InstanceSpread = 14,
    Mode = 15,
}

const PARAM_COUNT: usize = 16;

impl VstParam {
    fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(VstParam::FilterCutoff),
            1 => Some(VstParam::FilterResonance),
            2 => Some(VstParam::FilterMode),
            3 => Some(VstParam::ReverbMix),
            4 => Some(VstParam::DelayTime),
            5 => Some(VstParam::DelayFeedback),
            6 => Some(VstParam::EnvAttack),
            7 => Some(VstParam::EnvRelease),
            8 => Some(VstParam::Waveform),
            9 => Some(VstParam::MasterGain),
            10 => Some(VstParam::PrimeBase),
            11 => Some(VstParam::PrimePhase),
            12 => Some(VstParam::NnSensitivity),
            13 => Some(VstParam::InstanceDensity),
            14 => Some(VstParam::InstanceSpread),
            15 => Some(VstParam::Mode),
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            VstParam::FilterCutoff => "Filter Cutoff",
            VstParam::FilterResonance => "Filter Resonance",
            VstParam::FilterMode => "Filter Mode",
            VstParam::ReverbMix => "Reverb Mix",
            VstParam::DelayTime => "Delay Time",
            VstParam::DelayFeedback => "Delay Feedback",
            VstParam::EnvAttack => "Env Attack",
            VstParam::EnvRelease => "Env Release",
            VstParam::Waveform => "Waveform",
            VstParam::MasterGain => "Master Gain",
            VstParam::PrimeBase => "Prime Base",
            VstParam::PrimePhase => "Prime Phase",
            VstParam::NnSensitivity => "NN Sensitivity",
            VstParam::InstanceDensity => "Instance Density",
            VstParam::InstanceSpread => "Instance Spread",
            VstParam::Mode => "Mode",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            VstParam::FilterMode => "LP/HP/BP/Notch",
            VstParam::Waveform => "Sine/Saw/Sq/Tri",
            VstParam::Mode => "Synth/Smpl/FX/Ctrl",
            _ => "",
        }
    }
}

pub struct ModurustVstParams {
    params: Vec<AtomicFloat>,
}

impl Default for ModurustVstParams {
    fn default() -> Self {
        Self {
            params: (0..PARAM_COUNT).map(|_| AtomicFloat::new(0.0)).collect(),
        }
    }
}

impl PluginParameters for ModurustVstParams {
    fn get_parameter(&self, index: i32) -> f32 {
        let idx = index as usize;
        if idx < self.params.len() {
            self.params[idx].get()
        } else {
            0.0
        }
    }

    fn set_parameter(&self, index: i32, value: f32) {
        let idx = index as usize;
        if idx < self.params.len() {
            self.params[idx].set(value);
        }
    }

    fn get_parameter_name(&self, index: i32) -> String {
        VstParam::from_index(index as usize)
            .map(|p| p.name().to_string())
            .unwrap_or_default()
    }

    fn get_parameter_label(&self, index: i32) -> String {
        VstParam::from_index(index as usize)
            .map(|p| p.label().to_string())
            .unwrap_or_default()
    }

    fn get_parameter_text(&self, index: i32) -> String {
        let idx = index as usize;
        if idx < self.params.len() {
            format!("{:.3}", self.params[idx].get())
        } else {
            String::new()
        }
    }
}

pub struct ModurustVst {
    engine: Arc<Mutex<CoreEngine>>,
    cmd_tx: crossbeam_channel::Sender<EngineCommand>,
    params: Arc<ModurustVstParams>,
    sample_rate: f32,
    block_size: usize,
    running: Arc<AtomicBool>,
}

impl Default for ModurustVst {
    fn default() -> Self {
        Self::new(44100.0, 512)
    }
}

impl ModurustVst {
    pub fn new(sample_rate: f32, block_size: usize) -> Self {
        let config = EngineConfig::default();
        let (engine, cmd_tx) = CoreEngine::new(config, sample_rate);
        let engine = Arc::new(Mutex::new(engine));
        let params = Arc::new(ModurustVstParams::default());
        let running = Arc::new(AtomicBool::new(true));

        {
            let mut eng = engine.lock().unwrap();
            eng.master_gain = 0.8;
            eng.filter_cutoff_base = 1200.0;
        }

        Self {
            engine: engine.clone(),
            cmd_tx,
            params,
            sample_rate,
            block_size,
            running: running.clone(),
        }
    }

    fn start_udp_listener(&mut self) {
        let engine = self.engine.clone();
        let cmd_tx = self.cmd_tx.clone();
        let running = self.running.clone();

        let _handle = thread::spawn(move || {
            let socket = match UdpSocket::bind(format!("127.0.0.1:{}", UDP_CONTROL_PORT)) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[vst-udp] failed to bind UDP control port {}: {}", UDP_CONTROL_PORT, e);
                    return;
                }
            };
            socket.set_read_timeout(Some(Duration::from_millis(100))).ok();
            eprintln!("[vst-udp] listening on UDP {} for M4L control", UDP_CONTROL_PORT);

            let mut buf = [0u8; 1024];
            while running.load(Ordering::Relaxed) {
                match socket.recv_from(&mut buf) {
                    Ok((len, _)) => {
                        if let Ok(text) = std::str::from_utf8(&buf[..len]) {
                            Self::process_udp_text(text, &cmd_tx, &engine);
                        }
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(1));
                    }
                }
            }
            eprintln!("[vst-udp] UDP listener stopped");
        });
    }

    fn process_udp_text(text: &str, cmd_tx: &crossbeam_channel::Sender<EngineCommand>, _engine: &Arc<Mutex<CoreEngine>>) {
        let text = text.trim();
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        match parts[0] {
            "noteon" => {
                if parts.len() >= 3 {
                    if let (Ok(note), Ok(vel)) = (parts[1].parse::<u8>(), parts[2].parse::<u8>()) {
                        let _ = cmd_tx.send(EngineCommand::NoteOn { note, velocity: vel as f32 / 127.0 });
                    }
                }
            }
            "noteoff" => {
                if parts.len() >= 2 {
                    if let Ok(note) = parts[1].parse::<u8>() {
                        let _ = cmd_tx.send(EngineCommand::NoteOff { note });
                    }
                }
            }
            "alloff" => {
                let _ = cmd_tx.send(EngineCommand::AllNotesOff);
            }
            "prime" => {
                if parts.len() >= 2 {
                    if let Ok(base) = parts[1].parse::<u32>() {
                        let _ = cmd_tx.send(EngineCommand::SetPrimeBase(base));
                    }
                }
            }
            "phase" => {
                if parts.len() >= 2 {
                    if let Ok(phase) = parts[1].parse::<f32>() {
                        let _ = cmd_tx.send(EngineCommand::SetPrimePhase(phase));
                    }
                }
            }
            "sens" => {
                if parts.len() >= 2 {
                    if let Ok(sens) = parts[1].parse::<f32>() {
                        let _ = cmd_tx.send(EngineCommand::SetNnSensitivity(sens));
                    }
                }
            }
            "density" => {
                if parts.len() >= 2 {
                    if let Ok(density) = parts[1].parse::<u32>() {
                        let _ = cmd_tx.send(EngineCommand::SetInstanceDensity(density));
                    }
                }
            }
            "spread" => {
                if parts.len() >= 2 {
                    if let Ok(spread) = parts[1].parse::<f32>() {
                        let _ = cmd_tx.send(EngineCommand::SetInstanceSpread(spread));
                    }
                }
            }
            "mode" => {
                if parts.len() >= 2 {
                    let mode = match parts[1] {
                        "sampler" => EngineMode::Sampler,
                        "effect" => EngineMode::Effect,
                        "controller" => EngineMode::Controller,
                        _ => EngineMode::Synth,
                    };
                    let _ = cmd_tx.send(EngineCommand::SetMode(mode));
                }
            }
            "param" => {
                if parts.len() >= 3 {
                    if let Ok(value) = parts[2].parse::<f32>() {
                        let param = match parts[1] {
                            "filter_cutoff" => Some(Param::FilterCutoff),
                            "filter_resonance" => Some(Param::FilterResonance),
                            "filter_mode" => Some(Param::FilterMode),
                            "reverb_mix" => Some(Param::ReverbMix),
                            "delay_time" => Some(Param::DelayTime),
                            "delay_feedback" => Some(Param::DelayFeedback),
                            "env_attack" => Some(Param::EnvAttack),
                            "env_release" => Some(Param::EnvRelease),
                            "waveform" => Some(Param::Waveform),
                            "master_gain" => Some(Param::MasterGain),
                            _ => None,
                        };
                        if let Some(p) = param {
                            let _ = cmd_tx.send(EngineCommand::SetParam(p, value));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn filter_mode_from_norm(v: f32) -> FilterMode {
        match (v * 3.0).round() as i32 {
            1 => FilterMode::HighPass,
            2 => FilterMode::BandPass,
            3 => FilterMode::Notch,
            _ => FilterMode::LowPass,
        }
    }

    fn waveform_from_norm(v: f32) -> Waveform {
        match (v * 3.0).round() as i32 {
            1 => Waveform::Saw,
            2 => Waveform::Square,
            3 => Waveform::Triangle,
            _ => Waveform::Sine,
        }
    }

    fn mode_from_norm(v: f32) -> EngineMode {
        match (v * 3.0).round() as i32 {
            1 => EngineMode::Sampler,
            2 => EngineMode::Effect,
            3 => EngineMode::Controller,
            _ => EngineMode::Synth,
        }
    }

    fn apply_param_to_engine(&self, index: usize, value: f32) {
        let mut eng = self.engine.lock().unwrap();
        match VstParam::from_index(index) {
            Some(VstParam::FilterCutoff) => {
                eng.set_param(Param::FilterCutoff, value.clamp(0.0, 1.0));
            }
            Some(VstParam::FilterResonance) => {
                eng.set_param(Param::FilterResonance, value.clamp(0.0, 1.0));
            }
            Some(VstParam::FilterMode) => {
                let mode = Self::filter_mode_from_norm(value);
                eng.filter.set_mode(mode);
            }
            Some(VstParam::ReverbMix) => {
                eng.set_param(Param::ReverbMix, value.clamp(0.0, 1.0));
            }
            Some(VstParam::DelayTime) => {
                eng.set_param(Param::DelayTime, value.clamp(0.0, 1.0));
            }
            Some(VstParam::DelayFeedback) => {
                eng.set_param(Param::DelayFeedback, value.clamp(0.0, 1.0));
            }
            Some(VstParam::EnvAttack) => {
                eng.set_param(Param::EnvAttack, value.clamp(0.0, 1.0));
            }
            Some(VstParam::EnvRelease) => {
                eng.set_param(Param::EnvRelease, value.clamp(0.0, 1.0));
            }
            Some(VstParam::Waveform) => {
                let wf = Self::waveform_from_norm(value);
                eng.osc.waveform = wf;
            }
            Some(VstParam::MasterGain) => {
                eng.master_gain = value.clamp(0.0, 1.0);
            }
            Some(VstParam::PrimeBase) => {
                let base = (2.0 + value.clamp(0.0, 1.0) * 28.0) as u32;
                let _ = self.cmd_tx.send(EngineCommand::SetPrimeBase(base));
            }
            Some(VstParam::PrimePhase) => {
                let _ = self.cmd_tx.send(EngineCommand::SetPrimePhase(value));
            }
            Some(VstParam::NnSensitivity) => {
                let _ = self.cmd_tx.send(EngineCommand::SetNnSensitivity(value));
            }
            Some(VstParam::InstanceDensity) => {
                let d = (1.0 + value.clamp(0.0, 1.0) * 63.0) as u32;
                let _ = self.cmd_tx.send(EngineCommand::SetInstanceDensity(d));
            }
            Some(VstParam::InstanceSpread) => {
                let _ = self.cmd_tx.send(EngineCommand::SetInstanceSpread(value));
            }
            Some(VstParam::Mode) => {
                let m = Self::mode_from_norm(value);
                let _ = self.cmd_tx.send(EngineCommand::SetMode(m));
            }
            None => {}
        }
    }

    fn sync_params(&mut self) {
        for i in 0..PARAM_COUNT {
            let v = self.params.get_parameter(i as i32);
            self.apply_param_to_engine(i, v);
        }
    }

    fn start_udp_status_broadcaster(&self) {
        let engine = self.engine.clone();
        let running = self.running.clone();

        let _handle = thread::spawn(move || {
            let socket = match UdpSocket::bind("127.0.0.1:0") {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[vst-status] failed to bind socket: {}", e);
                    return;
                }
            };
            eprintln!("[vst-status] broadcasting engine status to port {}", UDP_STATUS_PORT);

            while running.load(Ordering::Relaxed) {
                if let Ok(eng) = engine.lock() {
                    let mode = match eng.config.mode.as_str() {
                        "sampler" => 1u8,
                        "effect" => 2u8,
                        "controller" => 3u8,
                        _ => 0u8,
                    };
                    let peak = eng.output_peak;
                    let base = eng.config.prime_base;
                    let phase = eng.config.prime_phase;
                    let sens = eng.config.nn_sensitivity;
                    let density = eng.config.instance_density;
                    let spread = eng.config.instance_spread;
                    let voices = eng.pool.active_count();

                    let text_packets = [
                        format!("mode {}", if mode == 1 { "sampler" } else if mode == 2 { "effect" } else if mode == 3 { "controller" } else { "synth" }),
                        format!("peak {:.5}", peak),
                        format!("prime_base {}", base),
                        format!("phase {:.5}", phase),
                        format!("sens {:.5}", sens),
                        format!("density {}", density),
                        format!("spread {:.5}", spread),
                        format!("voices {}", voices),
                    ];

                    for text in &text_packets {
                        let _ = socket.send_to(text.as_bytes(), ("127.0.0.1", UDP_STATUS_PORT));
                    }
                }
                thread::sleep(Duration::from_millis(50));
            }
            eprintln!("[vst-status] broadcaster stopped");
        });
    }
}

impl Plugin for ModurustVst {
    fn get_info(&self) -> Info {
        Info {
            name: "Modurust Modurust".to_string(),
            unique_id: PLUGIN_UNIQUE_ID as i32,
            category: vst::plugin::Category::Synth,
            inputs: 0,
            outputs: 2,
            parameters: PARAM_COUNT as i32,
            ..Default::default()
        }
    }

    fn new(_host: vst::plugin::HostCallback) -> Self {
        let mut plugin = Self::default();
        plugin.start_udp_listener();
        plugin
    }

    fn init(&mut self) {
        self.sync_params();
        self.start_udp_status_broadcaster();
        eprintln!("[vst] Modurust Modurust initialized with UDP control on port {}, status on port {}", UDP_CONTROL_PORT, UDP_STATUS_PORT);
    }

    fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate;
        let mut eng = self.engine.lock().unwrap();
        eng.sample_rate = rate;
    }

    fn set_block_size(&mut self, size: i64) {
        self.block_size = size.max(1) as usize;
    }

    fn process_events(&mut self, events: &vst::api::Events) {
        for event in events.events() {
            if let vst::event::Event::Midi(midi) = event {
                if midi.data.len() < 2 {
                    continue;
                }
                let status = midi.data[0] & 0xF0;
                match status {
                    0x90 => {
                        let note = midi.data[1];
                        let vel = if midi.data.len() > 2 { midi.data[2] } else { 100 };
                        if vel == 0 {
                            let _ = self.cmd_tx.send(EngineCommand::NoteOff { note });
                        } else {
                            let _ = self.cmd_tx.send(EngineCommand::NoteOn {
                                note,
                                velocity: vel as f32 / 127.0,
                            });
                        }
                    }
                    0x80 => {
                        let note = midi.data[1];
                        let _ = self.cmd_tx.send(EngineCommand::NoteOff { note });
                    }
                    _ => {}
                }
            }
        }
    }

    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        self.sync_params();

        let (inputs, mut outputs) = buffer.split();
        let frames = outputs[0].len();
        if frames == 0 {
            return;
        }

        let mut scratch_in: Vec<f32> = vec![0.0; frames];
        if inputs.len() > 0 {
            let in_ch = &inputs[0];
            for (i, s) in scratch_in.iter_mut().enumerate() {
                *s = in_ch.get(i).copied().unwrap_or(0.0);
            }
        }
        let mut scratch_out: Vec<f32> = vec![0.0; frames];
        {
            let mut eng = self.engine.lock().unwrap();
            eng.process_block(&mut scratch_out);
        }
        for i in 0..outputs.len() {
            let ch = &mut outputs[i];
            for (j, s) in ch.iter_mut().enumerate() {
                *s = scratch_out.get(j).copied().unwrap_or(0.0);
            }
        }
    }

    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        Arc::clone(&self.params) as Arc<dyn PluginParameters>
    }
}