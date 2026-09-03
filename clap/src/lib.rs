use clap_sys::entry::clap_plugin_entry;
use clap_sys::plugin::{clap_plugin, clap_plugin_descriptor};
use clap_sys::plugin_factory::{clap_plugin_factory};
use clap_sys::host::clap_host;
use clap_sys::process::{clap_process, clap_process_status, CLAP_PROCESS_ERROR, CLAP_PROCESS_CONTINUE};
use clap_sys::version::{clap_version, CLAP_VERSION_MAJOR, CLAP_VERSION_MINOR, CLAP_VERSION_REVISION};
use modurust_core::{CoreEngine, EngineConfig, EngineCommand};
use std::sync::{Arc, Mutex};
use crossbeam_channel::Sender;
use std::ptr;
use std::os::raw::c_char;
use std::os::raw::c_void;

const ID_C: &[u8] = b"modurust.clap\0";
const NAME_C: &[u8] = b"Modurust CLAP\0";
const VENDOR_C: &[u8] = b"Modurust\0";
const VERSION_C: &[u8] = b"0.1.0\0";
const DESCRIPTION_C: &[u8] = b"Modurust CLAP plugin\0";

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

struct PluginData {
    engine: Arc<Mutex<CoreEngine>>,
    cmd_tx: Sender<EngineCommand>,
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
        Self { engine, cmd_tx }
    }
}

extern "C" fn plugin_init(_plugin_path: *const c_char) -> bool {
    true
}

extern "C" fn plugin_deinit() {
}

extern "C" fn get_factory(factory_id: *const c_char) -> *const c_void {
    if factory_id.is_null() {
        return ptr::null();
    }
    
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

extern "C" fn clap_plugin_stop_processing_entry(_plugin: *const clap_plugin) {
}

extern "C" fn clap_plugin_reset_entry(plugin: *const clap_plugin) {
    unsafe {
        let data = &mut *((*plugin).plugin_data as *mut PluginData);
        let _ = data.cmd_tx.send(EngineCommand::AllNotesOff);
    }
}

extern "C" fn clap_plugin_process_entry(plugin: *const clap_plugin, process: *const clap_process) -> clap_process_status {
    if process.is_null() {
        return CLAP_PROCESS_ERROR;
    }

    unsafe {
        let data = &mut *((*plugin).plugin_data as *mut PluginData);
        let frames_count = (*process).frames_count as usize;

        if frames_count == 0 {
            return CLAP_PROCESS_CONTINUE;
        }

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
