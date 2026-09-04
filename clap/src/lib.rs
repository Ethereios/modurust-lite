use clap_sys::entry::clap_plugin_entry;
use clap_sys::plugin::{clap_plugin, clap_plugin_descriptor};
use clap_sys::plugin_factory::{clap_plugin_factory};
use clap_sys::host::clap_host;
use clap_sys::process::{clap_process, clap_process_status, CLAP_PROCESS_ERROR, CLAP_PROCESS_CONTINUE};
use clap_sys::version::{clap_version, CLAP_VERSION_MAJOR, CLAP_VERSION_MINOR, CLAP_VERSION_REVISION};
use modurust_core::{CoreEngine, EngineConfig, EngineCommand, EngineMode};
use std::sync::{Arc, Mutex};
use crossbeam_channel::Sender;
use std::ptr;
use std::os::raw::c_char;
use std::os::raw::c_void;
use std::thread;
use std::time::Duration;
use std::net::UdpSocket;

const ID_C: &[u8] = b"modurust.clap\0";
const NAME_C: &[u8] = b"Modurust\0";
const VENDOR_C: &[u8] = b"Modurust\0";
const VERSION_C: &[u8] = b"0.1.0\0";
const DESCRIPTION_C: &[u8] = b"Modurust modular DSP engine with prime/NN control\0";

const UDP_CONTROL_PORT: u16 = 9000;
const UDP_STATUS_PORT: u16 = 9002;

struct Features(pub [*const c_char; 3]);
unsafe impl Sync for Features {}

static FEATURES: Features = Features([
    clap_sys::plugin_features::CLAP_PLUGIN_FEATURE_INSTRUMENT.as_ptr(),
    clap_sys::plugin_features::CLAP_PLUGIN_FEATURE_AUDIO_EFFECT.as_ptr(),
    ptr::null(),
]);

#[repr(C)]
struct PluginDescriptor {
    clap_version: clap_version,
    id: *const c_char,
    name: *const c_char,
    vendor: *const c_char,
    url: *const c_char,
    manual_url: *const c_char,
    support_url: *const c_char,
    version: *const c_char,
    description: *const c_char,
    features: *const *const c_char,
}

static PLUGIN_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    clap_version: clap_version {
        major: CLAP_VERSION_MAJOR,
        minor: CLAP_VERSION_MINOR,
        revision: CLAP_VERSION_REVISION,
    },
    id: ID_C.as_ptr() as *const c_char,
    name: NAME_C.as_ptr() as *const c_char,
    vendor: VENDOR_C.as_ptr() as *const c_char,
    url: ptr::null(),
    manual_url: ptr::null(),
    support_url: ptr::null(),
    version: VERSION_C.as_ptr() as *const c_char,
    description: DESCRIPTION_C.as_ptr() as *const c_char,
    features: FEATURES.0.as_ptr(),
};

unsafe impl Sync for PluginDescriptor {}

pub struct PluginData {
    engine: Arc<Mutex<CoreEngine>>,
    cmd_tx: Sender<EngineCommand>,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl PluginData {
    fn new() -> Self {
        let config = EngineConfig::default();
        let (engine, cmd_tx) = CoreEngine::new(config, 44100.0);
        let engine = Arc::new(Mutex::new(engine));
        {
            let mut eng = engine.lock().unwrap();
            eng.master_gain = 0.8;
            eng.filter_cutoff_base = 1200.0;
        }
        Self {
            engine,
            cmd_tx,
            running: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        }
    }

    fn start_udp_listener(running: Arc<std::sync::atomic::AtomicBool>, cmd_tx: Sender<EngineCommand>) {
        let _ = thread::spawn(move || {
            let socket = match UdpSocket::bind(format!("127.0.0.1:{}", UDP_CONTROL_PORT)) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[clap-udp] failed to bind UDP control port {}: {}", UDP_CONTROL_PORT, e);
                    return;
                }
            };
            let _ = socket.set_read_timeout(Some(Duration::from_millis(100)));
            eprintln!("[clap-udp] listening on UDP {} for M4L control", UDP_CONTROL_PORT);

            let mut buf = [0u8; 1024];
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                match socket.recv_from(&mut buf) {
                    Ok((len, _)) => {
                        if let Ok(text) = std::str::from_utf8(&buf[..len]) {
                            Self::process_udp_text(text, &cmd_tx);
                        }
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(1));
                    }
                }
            }
            eprintln!("[clap-udp] UDP listener stopped");
        });
    }

    fn start_udp_status_broadcaster(engine: Arc<Mutex<CoreEngine>>, running: Arc<std::sync::atomic::AtomicBool>) {
        let _ = thread::spawn(move || {
            let socket = match UdpSocket::bind(format!("127.0.0.1:{}", UDP_STATUS_PORT)) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[clap-status] failed to bind UDP status port {}: {}", UDP_STATUS_PORT, e);
                    return;
                }
            };
            eprintln!("[clap-status] broadcasting engine status to port {}", UDP_STATUS_PORT);

            while running.load(std::sync::atomic::Ordering::Relaxed) {
                if let Ok(eng) = engine.lock() {
                    let mode = match eng.config.mode.as_str() {
                        "sampler" => "mode sampler",
                        "effect" => "mode effect",
                        "controller" => "mode controller",
                        _ => "mode synth",
                    };
                    let peak = format!("peak {:.5}", eng.output_peak);
                    let base = format!("prime_base {}", eng.config.prime_base);
                    let phase = format!("phase {:.5}", eng.config.prime_phase);
                    let sens = format!("sens {:.5}", eng.config.nn_sensitivity);
                    let voices = format!("voices {}", eng.pool.active_count());

                    let _ = socket.send_to(mode.as_bytes(), ("127.0.0.1", UDP_STATUS_PORT));
                    let _ = socket.send_to(peak.as_bytes(), ("127.0.0.1", UDP_STATUS_PORT));
                    let _ = socket.send_to(base.as_bytes(), ("127.0.0.1", UDP_STATUS_PORT));
                    let _ = socket.send_to(phase.as_bytes(), ("127.0.0.1", UDP_STATUS_PORT));
                    let _ = socket.send_to(sens.as_bytes(), ("127.0.0.1", UDP_STATUS_PORT));
                    let _ = socket.send_to(voices.as_bytes(), ("127.0.0.1", UDP_STATUS_PORT));
                }
                thread::sleep(Duration::from_millis(50));
            }
            eprintln!("[clap-status] broadcaster stopped");
        });
    }

    fn process_udp_text(text: &str, cmd_tx: &Sender<EngineCommand>) {
        let text = text.trim();
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.is_empty() { return; }

        match parts[0] {
            "noteon" => {
                if parts.len() >= 3 {
                    if let (Ok(note), Ok(vel)) = (parts[1].parse::<u8>(), parts[2].parse::<u8>()) {
                        let _ = cmd_tx.send(EngineCommand::NoteOn { note, velocity: vel as f32 / 127.0 });
                    }
                }
            }
            "noteoff" if parts.len() >= 2 => {
                if let Ok(note) = parts[1].parse::<u8>() {
                    let _ = cmd_tx.send(EngineCommand::NoteOff { note });
                }
            }
            "alloff" => { let _ = cmd_tx.send(EngineCommand::AllNotesOff); }
            "prime" => {
                if parts.len() >= 2 { if let Ok(base) = parts[1].parse::<u32>() { let _ = cmd_tx.send(EngineCommand::SetPrimeBase(base)); } }
            }
            "phase" => {
                if parts.len() >= 2 { if let Ok(phase) = parts[1].parse::<f32>() { let _ = cmd_tx.send(EngineCommand::SetPrimePhase(phase)); } }
            }
            "sens" => {
                if parts.len() >= 2 { if let Ok(sens) = parts[1].parse::<f32>() { let _ = cmd_tx.send(EngineCommand::SetNnSensitivity(sens)); } }
            }
            "density" => {
                if parts.len() >= 2 { if let Ok(d) = parts[1].parse::<u32>() { let _ = cmd_tx.send(EngineCommand::SetInstanceDensity(d)); } }
            }
            "spread" => {
                if parts.len() >= 2 { if let Ok(s) = parts[1].parse::<f32>() { let _ = cmd_tx.send(EngineCommand::SetInstanceSpread(s)); } }
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
                        let cmd = match parts[1] {
                            "filter_cutoff" => Some(EngineCommand::SetParam(modurust_core::engine::Param::FilterCutoff, value)),
                            "filter_resonance" => Some(EngineCommand::SetParam(modurust_core::engine::Param::FilterResonance, value)),
                            "filter_mode" => Some(EngineCommand::SetParam(modurust_core::engine::Param::FilterMode, value)),
                            "reverb_mix" => Some(EngineCommand::SetParam(modurust_core::engine::Param::ReverbMix, value)),
                            "delay_time" => Some(EngineCommand::SetParam(modurust_core::engine::Param::DelayTime, value)),
                            "delay_feedback" => Some(EngineCommand::SetParam(modurust_core::engine::Param::DelayFeedback, value)),
                            "env_attack" => Some(EngineCommand::SetParam(modurust_core::engine::Param::EnvAttack, value)),
                            "env_release" => Some(EngineCommand::SetParam(modurust_core::engine::Param::EnvRelease, value)),
                            "waveform" => Some(EngineCommand::SetParam(modurust_core::engine::Param::Waveform, value)),
                            "master_gain" => Some(EngineCommand::SetParam(modurust_core::engine::Param::MasterGain, value)),
                            _ => None,
                        };
                        if let Some(cmd) = cmd { let _ = cmd_tx.send(cmd); }
                    }
                }
            }
            _ => {}
        }
    }
}

extern "C" fn plugin_init(_plugin_path: *const c_char) -> bool {
    true
}

extern "C" fn plugin_deinit() {}

extern "C" fn get_factory(factory_id: *const c_char) -> *const c_void {
    if factory_id.is_null() { return ptr::null(); }
    let id = unsafe { std::ffi::CStr::from_ptr(factory_id) };
    if id.to_bytes_with_nul() == clap_sys::plugin_factory::CLAP_PLUGIN_FACTORY_ID.to_bytes_with_nul() {
        &FACTORY as *const _ as *const c_void
    } else {
        ptr::null()
    }
}

static FACTORY: clap_plugin_factory = clap_plugin_factory {
    get_plugin_count: Some(clap_plugin_factory_get_plugin_count),
    get_plugin_descriptor: Some(clap_plugin_factory_get_plugin_descriptor),
    create_plugin: Some(clap_plugin_factory_create_plugin),
};

#[no_mangle]
pub static clap_entry: clap_plugin_entry = clap_plugin_entry {
    clap_version: clap_version {
        major: CLAP_VERSION_MAJOR,
        minor: CLAP_VERSION_MINOR,
        revision: CLAP_VERSION_REVISION,
    },
    init: Some(plugin_init),
    deinit: Some(plugin_deinit),
    get_factory: Some(get_factory),
};

extern "C" fn clap_plugin_init_entry(plugin: *const clap_plugin) -> bool {
    unsafe {
        let data = &mut *((*plugin).plugin_data as *mut PluginData);
        let mut eng = data.engine.lock().unwrap();
        eng.master_gain = 0.8;
        eng.filter_cutoff_base = 1200.0;
    }
    true
}

extern "C" fn clap_plugin_destroy_entry(plugin: *const clap_plugin) {
    unsafe {
        let data = &mut *((*plugin).plugin_data as *mut PluginData);
        let _ = data.cmd_tx.send(EngineCommand::AllNotesOff);
        data.running.store(false, std::sync::atomic::Ordering::Relaxed);
        drop(Box::from_raw(data as *mut PluginData));
    }
}

extern "C" fn clap_plugin_activate_entry(
    plugin: *const clap_plugin,
    sample_rate: f64,
    _min_frames_count: u32,
    _max_frames_count: u32,
) -> bool {
    unsafe {
        let data = &mut *((*plugin).plugin_data as *mut PluginData);
        let mut eng = data.engine.lock().unwrap();
        eng.sample_rate = sample_rate as f32;
    }
    true
}

extern "C" fn clap_plugin_deactivate_entry(plugin: *const clap_plugin) {
    unsafe {
        let data = &mut *((*plugin).plugin_data as *mut PluginData);
        let _ = data.cmd_tx.send(EngineCommand::AllNotesOff);
    }
}

extern "C" fn clap_plugin_start_processing_entry(_plugin: *const clap_plugin) -> bool {
    true
}

extern "C" fn clap_plugin_stop_processing_entry(_plugin: *const clap_plugin) {}

extern "C" fn clap_plugin_reset_entry(plugin: *const clap_plugin) {
    unsafe {
        let data = &mut *((*plugin).plugin_data as *mut PluginData);
        let _ = data.cmd_tx.send(EngineCommand::AllNotesOff);
    }
}

extern "C" fn clap_plugin_process_entry(plugin: *const clap_plugin, process: *const clap_process) -> clap_process_status {
    if process.is_null() { return CLAP_PROCESS_ERROR; }

    unsafe {
        let data = &mut *((*plugin).plugin_data as *mut PluginData);
        let frames_count = (*process).frames_count as usize;

        if frames_count == 0 { return CLAP_PROCESS_CONTINUE; }

        let mut eng = data.engine.lock().unwrap();
        let mut scratch_out: Vec<f32> = vec![0.0; frames_count];
        eng.process_block(&mut scratch_out);

        let out_buf = (*process).audio_outputs;
        if !out_buf.is_null() {
            let ch = (*out_buf).data32;
            if !ch.is_null() {
                let ch_ptr = *ch as *mut f32;
                let ch_slice = std::slice::from_raw_parts_mut(ch_ptr, frames_count);
                for (i, sample) in ch_slice.iter_mut().enumerate() {
                    *sample = scratch_out.get(i).copied().unwrap_or(0.0);
                }
            }
        }
    }

    CLAP_PROCESS_CONTINUE
}

#[no_mangle]
pub extern "C" fn clap_plugin_factory_get_plugin_count(_factory: *const clap_plugin_factory) -> u32 {
    1
}

extern "C" fn clap_plugin_factory_get_plugin_descriptor(
    _factory: *const clap_plugin_factory,
    index: u32,
) -> *const clap_plugin_descriptor {
    if index == 0 {
        &PLUGIN_DESCRIPTOR as *const PluginDescriptor as *const clap_plugin_descriptor
    } else {
        ptr::null()
    }
}

extern "C" fn clap_plugin_factory_create_plugin(
    _factory: *const clap_plugin_factory,
    _host: *const clap_host,
    _plugin_id: *const c_char,
) -> *const clap_plugin {
    let data = PluginData::new();
    let plugin_data = Box::into_raw(Box::new(data)) as *mut c_void;

    let data_ref = unsafe { &*(plugin_data as *const PluginData) };
    PluginData::start_udp_listener(data_ref.running.clone(), data_ref.cmd_tx.clone());
    PluginData::start_udp_status_broadcaster(data_ref.engine.clone(), data_ref.running.clone());

    let plugin = Box::leak(Box::new(clap_plugin {
        desc: &PLUGIN_DESCRIPTOR as *const PluginDescriptor as *const clap_plugin_descriptor,
        plugin_data,
        init: Some(clap_plugin_init_entry),
        destroy: Some(clap_plugin_destroy_entry),
        activate: Some(clap_plugin_activate_entry),
        deactivate: Some(clap_plugin_deactivate_entry),
        start_processing: Some(clap_plugin_start_processing_entry),
        stop_processing: Some(clap_plugin_stop_processing_entry),
        reset: Some(clap_plugin_reset_entry),
        process: Some(clap_plugin_process_entry),
        get_extension: None,
        on_main_thread: None,
    }));

    plugin
}