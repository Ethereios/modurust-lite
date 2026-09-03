//! UI Module for HexoDSP DAW
//!
//! This module exports the revolutionary three-view UI system:
//! - Arrangement View: Traditional DAW timeline/arrangement
//! - Live View: Real-time performance interface
//! - Node View: Modular node-based patching

// Declare the submodules

mod egui_ui_full;
pub mod hexagonal_node_view;
pub mod bevy_egui_ui;

// Re-export the full-featured egui UI implementation
pub use egui_ui_full::*;
pub use bevy_egui_ui::*;

// Re-export all sophisticated UI components and state management from the full UI
pub use egui_ui_full::UIViewMode;
pub use egui_ui_full::UiState;
pub use egui_ui_full::ArrangementViewState;
pub use egui_ui_full::LiveViewState;
pub use egui_ui_full::NodeViewState;
pub use egui_ui_full::VisualFeedbackSettings;
pub use egui_ui_full::AutomationCurve;
pub use egui_ui_full::AutomationPoint;
pub use egui_ui_full::TrackRoute;
pub use egui_ui_full::ClipOperation;
pub use egui_ui_full::MegaPlugin;
pub use egui_ui_full::ModulationRoute;
pub use egui_ui_full::NodePosition;
pub use egui_ui_full::NodeConnection;
pub use egui_ui_full::NodeParameter;
pub use egui_ui_full::NodePreset;
pub use egui_ui_full::NuweShader;
pub use egui_ui_full::ISFPlugin;
