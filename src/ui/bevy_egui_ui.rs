use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use crate::ui::{UiState, ArrangementViewState, LiveViewState, NodeViewState};
use crate::audio_engine::HexoDSPEngine;
use crate::audio_engine::bridge::AudioParamMessage;
use crate::event_queue::EventQueue;
use std::sync::Arc;
use std::io::Write;

// Thin wrappers so large UI states can live as Bevy resources
#[derive(Resource, Default)]
pub struct UiStateRes(pub UiState);

#[derive(Resource, Default)]
struct ArrangementViewStateRes(pub ArrangementViewState);

#[derive(Resource, Default)]
struct LiveViewStateRes(pub LiveViewState);

#[derive(Resource, Default)]
struct NodeViewStateRes(pub NodeViewState);

pub struct HexoDSPUiPlugin;

impl Plugin for HexoDSPUiPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(UiStateRes::default())
            .insert_resource(ArrangementViewStateRes::default())
            .insert_resource(LiveViewStateRes::default())
            .insert_resource(NodeViewStateRes::default())
            .add_plugins(EguiPlugin::default())
            .add_systems(Startup, (setup_camera, setup_engine))
            .add_systems(EguiPrimaryContextPass, ui_system)
            .add_systems(Update, save_ui_state_on_exit);
    }
}

fn setup_camera(mut commands: Commands) {
    // bevy_egui requires at least one camera
    commands.spawn(Camera2d);
}

fn setup_engine(
    mut ui_state: ResMut<UiStateRes>,
) {
    // Initialize audio engine and share its bridge with the UI state
    let event_queue = Arc::new(EventQueue::new());
    let engine = HexoDSPEngine::new(event_queue.clone()).expect("Failed to initialize audio engine");

    // Clone the shared bridge Arc for UI transport controls
    let bridge_arc = engine.bridge.clone();
    ui_state.0.audio_bridge = Some(bridge_arc);

    // Start audio (output stream) – engine owns AudioIO
    // Leak the engine to keep it alive on the main thread (non-Send type)
    let engine_static: &'static mut HexoDSPEngine = Box::leak(Box::new(engine));
    if let Err(e) = engine_static.start() {
        eprintln!("Failed to start audio engine: {}", e);
    }

    // Safe defaults: disable input monitoring on startup to prevent feedback
    if let Some(bridge_arc) = ui_state.0.audio_bridge.clone() {
        if let Ok(bridge) = bridge_arc.lock() {
            let _ = bridge.send_param(AudioParamMessage::SetInputMonitoring(false));
            // Initialize audible test tone to match UI defaults and start playback
            let _ = bridge.send_param(AudioParamMessage::MasterVolume(ui_state.0.master_volume));
            let _ = bridge.send_param(AudioParamMessage::MasterPan(ui_state.0.master_pan));
            let _ = bridge.send_param(AudioParamMessage::Play);
        }
    }
}

fn ui_system(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiStateRes>,
    mut arrangement_state: ResMut<ArrangementViewStateRes>,
    mut live_state: ResMut<LiveViewStateRes>,
    mut node_state: ResMut<NodeViewStateRes>,
) {
    if let Ok(ctx) = contexts.ctx_mut() {
        // Drive the full UI from the egui implementation using bevy_egui context
        // Wrap in a panic guard to prevent red error screens from crashing the frame
        let ui_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::ui::egui_ui_full::ui_system(
                ctx,
                &mut ui_state.0,
                &mut arrangement_state.0,
                &mut live_state.0,
                &mut node_state.0,
            );
        }));

        if ui_result.is_err() {
            // Log the panic event for diagnostics without crashing the frame
            eprintln!("UI panic captured in bevy_egui_ui::ui_system; entering safe mode.");
            let _ = std::fs::create_dir_all("logs");
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            // Rate-limit log writes to avoid flooding the file on repeated panics
            let write_allowed = match std::fs::metadata("logs/ui_panic.log").and_then(|m| m.modified()) {
                Ok(mod_time) => std::time::SystemTime::now()
                    .duration_since(mod_time)
                    .map(|d| d.as_secs())
                    .unwrap_or(u64::MAX) > 2, // allow a write at most every 2 seconds
                Err(_) => true,
            };
            if write_allowed {
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("logs/ui_panic.log")
                {
                    let _ = writeln!(file, "ts={} UI panic captured; safe mode engaged", ts);
                }
            }
            // Show a friendly banner instead of a red panic overlay
            egui::TopBottomPanel::top("panic_banner").show(ctx, |ui| {
                ui.colored_label(
                    egui::Color32::from_rgb(220, 80, 80),
                    "UI encountered an error. Running in safe mode. Check logs.",
                );
            });
        }

        // Removed floating overlay: Audio Engine Monitor is now integrated in the main UI panels
    }
}

fn save_ui_state_on_exit(
    ui_state: Res<UiStateRes>,
    mut exit_events: MessageReader<bevy::app::AppExit>,
) {
    if exit_events.read().next().is_some() {
        if let Ok(json) = serde_json::to_string_pretty(&ui_state.0) {
            let _ = std::fs::write("ui_state.json", json);
        }
    }
}