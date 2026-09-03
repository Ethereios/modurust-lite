use egui;
use egui::{Pos2, Rect};
// Removed unused egui type imports
use crate::ui::hexagonal_node_view::HexNodeViewState;

// UI View Modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UIViewMode {
    #[default]
    Arrangement,
    Live,
    Node,
}

// Top-level UI state
#[derive(Default)]
pub struct UiState {
    pub view_mode: UIViewMode,
}

// Arrangement view state (simplified)
#[derive(Default)]
pub struct ArrangementViewState {
    pub timeline_position: f32,
    pub timeline_zoom: f32,
    pub snap_to_grid: bool,
}

// Live view state (simplified)
#[derive(Default)]
pub struct LiveViewState {
    pub bpm: f32,
    pub quantize: bool,
}

// Node view state integrates the hex node canvas
#[derive(Default)]
pub struct NodeViewState {
    pub patch_name: String,
    pub hex: HexNodeViewState,
}

// Full application composed of three views
#[derive(Default)]
pub struct HexoDSPApp {
    pub ui_state: UiState,
    pub arrangement_state: ArrangementViewState,
    pub live_state: LiveViewState,
    pub node_state: NodeViewState,
}

impl HexoDSPApp {
    pub fn update(&mut self, ctx: &egui::Context) {
        ui_system(ctx, &mut self.ui_state, &mut self.arrangement_state, &mut self.live_state, &mut self.node_state);
    }
}

// Main UI system: menu bar, view switcher, and content
pub fn ui_system(
    ctx: &egui::Context,
    ui_state: &mut UiState,
    arrangement_state: &mut ArrangementViewState,
    live_state: &mut LiveViewState,
    node_state: &mut NodeViewState,
) {
    // Top menu bar
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.menu_button("📁 File", |ui| {
                if ui.button("🆕 New Project").clicked() {}
                if ui.button("📂 Open Project").clicked() {}
                if ui.button("💾 Save Project").clicked() {}
                if ui.button("📤 Export").clicked() {}
            });
            ui.menu_button("✏️ Edit", |ui| {
                let _ = ui.button("↶ Undo");
                let _ = ui.button("↷ Redo");
            });
            ui.menu_button("📖 Docs", |ui| {
                ui.label("See docs/ for full UI plan");
            });

            ui.separator();
            ui.selectable_value(&mut ui_state.view_mode, UIViewMode::Arrangement, "Arrangement");
            ui.selectable_value(&mut ui_state.view_mode, UIViewMode::Live, "Live");
            ui.selectable_value(&mut ui_state.view_mode, UIViewMode::Node, "Node");
        });
    });

    // Central content
    egui::CentralPanel::default().show(ctx, |ui| {
        match ui_state.view_mode {
            UIViewMode::Arrangement => arrangement_view(ui, arrangement_state),
            UIViewMode::Live => live_view(ui, live_state),
            UIViewMode::Node => node_view(ui, node_state),
        }
    });
}

// Arrangement view content (simplified)
fn arrangement_view(ui: &mut egui::Ui, state: &mut ArrangementViewState) {
    ui.heading("🎼 Arrangement View");
    ui.horizontal(|ui| {
        ui.label("Timeline Position:");
        ui.add(egui::Slider::new(&mut state.timeline_position, 0.0..=1024.0));
        ui.label("Zoom:");
        ui.add(egui::Slider::new(&mut state.timeline_zoom, 0.1..=10.0));
        ui.checkbox(&mut state.snap_to_grid, "Snap to Grid");
    });
    ui.separator();
    ui.label("Tracks and clips go here (see docs)");
}

// Live performance view content (simplified)
fn live_view(ui: &mut egui::Ui, state: &mut LiveViewState) {
    ui.heading("🎛️ Live View");
    ui.horizontal(|ui| {
        ui.label("BPM:");
        ui.add(egui::Slider::new(&mut state.bpm, 40.0..=240.0));
        ui.checkbox(&mut state.quantize, "Quantize");
    });
    ui.separator();
    ui.label("Performance controls, clip launching, gesture input (see docs)");
}

// Node view using the hexagonal node canvas state
fn node_view(ui: &mut egui::Ui, state: &mut NodeViewState) {
    ui.heading("🔗 Node View - Modular Patching");
    ui.horizontal(|ui| {
        if ui.button("➕ Sine Osc").clicked() {
            let pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::new(100.0, 100.0));
            state.hex.add_node("Sine Osc", "generator.sine", pos);
        }
        if ui.button("➕ LPF").clicked() {
            let pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::new(220.0, 100.0));
            state.hex.add_node("LPF", "filter.lpf", pos);
        }
        if ui.button("➕ Delay").clicked() {
            let pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::new(340.0, 100.0));
            state.hex.add_node("Delay", "effect.delay", pos);
        }
    });

    // Canvas area
    let available = ui.available_size();
    let canvas_rect = Rect::from_min_size(ui.cursor().min, available);
    state.hex.draw(ui, canvas_rect);
    ui.allocate_rect(canvas_rect, egui::Sense::click());
}


// NOTE: Placeholder type stubs removed; real implementations provided above.

// The second implementation has been removed to fix the duplicate function error
