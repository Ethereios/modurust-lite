//! Bevy Web UI - Unified UI System for Desktop and Web
//!
//! This module provides a unified UI abstraction that works across:
//! - Desktop (Bevy + egui)
//! - Web (Bevy compiled to WebAssembly + HTML5 Canvas/WebGL)
//!
//! The UI system automatically detects the target platform and uses the
//! appropriate rendering backend while maintaining identical functionality.

use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::utils::HashMap;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{console, window, HtmlCanvasElement, WebGlRenderingContext};

#[cfg(not(target_arch = "wasm32"))]
use bevy_egui::{egui, EguiContext, EguiPlugin};

// UI Component Types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UIComponentType {
    Button,
    Slider,
    Label,
    Canvas,
    Panel,
    Window,
    Menu,
    Toolbar,
    StatusBar,
    Tree,
    List,
    Grid,
    Tabs,
}

// UI Events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UIEvent {
    Click { component_id: String, button: MouseButton },
    Drag { component_id: String, delta: Vec2 },
    ValueChanged { component_id: String, value: UIValue },
    KeyPress { key: KeyCode, modifiers: KeyModifiers },
    Resize { width: f32, height: f32 },
    Focus { component_id: String },
    Blur { component_id: String },
}

// UI Values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UIValue {
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
    Vec2(Vec2),
    Color(Color),
}

// UI Component Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIComponent {
    pub id: String,
    pub component_type: UIComponentType,
    pub position: Vec2,
    pub size: Vec2,
    pub properties: HashMap<String, UIValue>,
    pub children: Vec<String>, // Child component IDs
    pub parent: Option<String>, // Parent component ID
    pub visible: bool,
    pub enabled: bool,
    pub z_index: i32,
}

// Unified UI System
#[derive(Resource)]
pub struct UnifiedUI {
    components: HashMap<String, UIComponent>,
    event_queue: Vec<UIEvent>,
    focused_component: Option<String>,
    #[cfg(target_arch = "wasm32")]
    web_context: Option<WebUIContext>,
    #[cfg(not(target_arch = "wasm32"))]
    egui_context: Option<bevy_egui::EguiContext>,
}

#[cfg(target_arch = "wasm32")]
struct WebUIContext {
    canvas: HtmlCanvasElement,
    gl_context: WebGlRenderingContext,
    event_listeners: HashMap<String, js_sys::Function>,
}

impl Default for UnifiedUI {
    fn default() -> Self {
        Self {
            components: HashMap::new(),
            event_queue: Vec::new(),
            focused_component: None,
            #[cfg(target_arch = "wasm32")]
            web_context: None,
            #[cfg(not(target_arch = "wasm32"))]
            egui_context: None,
        }
    }
}

impl UnifiedUI {
    // Initialize the UI system based on platform
    pub fn init(&mut self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        {
            self.init_web_ui();
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            app.add_plugin(EguiPlugin);
            // egui context will be available after plugin init
        }

        // Initialize common UI components
        self.init_common_components();
    }

    #[cfg(target_arch = "wasm32")]
    fn init_web_ui(&mut self) {
        // Get canvas element
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let canvas = document
            .get_element_by_id("bevy-canvas")
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()
            .unwrap();

        // Get WebGL context
        let gl_context = canvas
            .get_context("webgl")
            .unwrap()
            .unwrap()
            .dyn_into::<WebGlRenderingContext>()
            .unwrap();

        // Setup event listeners
        let event_listeners = self.setup_web_event_listeners(&canvas);

        self.web_context = Some(WebUIContext {
            canvas,
            gl_context,
            event_listeners,
        });

        web_log("Web UI initialized");
    }

    #[cfg(target_arch = "wasm32")]
    fn setup_web_event_listeners(&self, canvas: &HtmlCanvasElement) -> HashMap<String, js_sys::Function> {
        let mut listeners = HashMap::new();

        // Mouse events
        let mouse_callback = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let ui_event = UIEvent::Click {
                component_id: "canvas".to_string(),
                button: match event.button() {
                    0 => MouseButton::Left,
                    1 => MouseButton::Middle,
                    2 => MouseButton::Right,
                    _ => MouseButton::Left,
                },
            };
            Self::queue_web_event(ui_event);
        }) as Box<dyn FnMut(_)>);

        canvas.add_event_listener_with_callback("click", mouse_callback.as_ref().unchecked_ref()).unwrap();
        listeners.insert("click".to_string(), mouse_callback.as_ref().clone());

        mouse_callback.forget(); // Leak to keep alive

        // Keyboard events
        let key_callback = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let key_code = match event.key().as_str() {
                " " => KeyCode::Space,
                "Enter" => KeyCode::Return,
                "Escape" => KeyCode::Escape,
                _ => return, // Ignore other keys for now
            };

            let ui_event = UIEvent::KeyPress {
                key: key_code,
                modifiers: KeyModifiers {
                    ctrl: event.ctrl_key(),
                    shift: event.shift_key(),
                    alt: event.alt_key(),
                },
            };
            Self::queue_web_event(ui_event);
        }) as Box<dyn FnMut(_)>);

        canvas.add_event_listener_with_callback("keydown", key_callback.as_ref().unchecked_ref()).unwrap();
        listeners.insert("keydown".to_string(), key_callback.as_ref().clone());

        key_callback.forget();

        listeners
    }

    #[cfg(target_arch = "wasm32")]
    fn queue_web_event(event: UIEvent) {
        // Send event to Rust side via WebAssembly
        unsafe {
            // This would call a WebAssembly export to queue the event
            // For now, we'll use a global queue
            if let Some(ui) = UNIFIED_UI.get() {
                ui.lock().unwrap().event_queue.push(event);
            }
        }
    }

    // Initialize common UI components that work across platforms
    fn init_common_components(&mut self) {
        // Main toolbar
        self.add_component(UIComponent {
            id: "main_toolbar".to_string(),
            component_type: UIComponentType::Toolbar,
            position: Vec2::new(0.0, 0.0),
            size: Vec2::new(800.0, 40.0),
            properties: HashMap::from([
                ("background_color".to_string(), UIValue::Color(Color::rgb(0.2, 0.2, 0.2))),
            ]),
            children: vec!["play_button".to_string(), "stop_button".to_string()],
            parent: None,
            visible: true,
            enabled: true,
            z_index: 100,
        });

        // Play button
        self.add_component(UIComponent {
            id: "play_button".to_string(),
            component_type: UIComponentType::Button,
            position: Vec2::new(10.0, 5.0),
            size: Vec2::new(30.0, 30.0),
            properties: HashMap::from([
                ("text".to_string(), UIValue::String("▶".to_string())),
                ("background_color".to_string(), UIValue::Color(Color::rgb(0.0, 0.8, 0.0))),
            ]),
            children: vec![],
            parent: Some("main_toolbar".to_string()),
            visible: true,
            enabled: true,
            z_index: 101,
        });

        // Stop button
        self.add_component(UIComponent {
            id: "stop_button".to_string(),
            component_type: UIComponentType::Button,
            position: Vec2::new(50.0, 5.0),
            size: Vec2::new(30.0, 30.0),
            properties: HashMap::from([
                ("text".to_string(), UIValue::String("⏹".to_string())),
                ("background_color".to_string(), UIValue::Color(Color::rgb(0.8, 0.0, 0.0))),
            ]),
            children: vec![],
            parent: Some("main_toolbar".to_string()),
            visible: true,
            enabled: true,
            z_index: 101,
        });

        // Main canvas area
        self.add_component(UIComponent {
            id: "main_canvas".to_string(),
            component_type: UIComponentType::Canvas,
            position: Vec2::new(0.0, 40.0),
            size: Vec2::new(800.0, 560.0),
            properties: HashMap::from([
                ("background_color".to_string(), UIValue::Color(Color::rgb(0.1, 0.1, 0.1))),
            ]),
            children: vec![],
            parent: None,
            visible: true,
            enabled: true,
            z_index: 1,
        });
    }

    // Add a UI component
    pub fn add_component(&mut self, component: UIComponent) {
        self.components.insert(component.id.clone(), component);
    }

    // Remove a UI component
    pub fn remove_component(&mut self, component_id: &str) {
        if let Some(component) = self.components.remove(component_id) {
            // Remove from parent's children list
            if let Some(parent_id) = &component.parent {
                if let Some(parent) = self.components.get_mut(parent_id) {
                    parent.children.retain(|id| id != component_id);
                }
            }

            // Remove children recursively
            for child_id in &component.children.clone() {
                self.remove_component(child_id);
            }
        }
    }

    // Update a component's properties
    pub fn update_component(&mut self, component_id: &str, updates: HashMap<String, UIValue>) {
        if let Some(component) = self.components.get_mut(component_id) {
            for (key, value) in updates {
                component.properties.insert(key, value);
            }
        }
    }

    // Get a component
    pub fn get_component(&self, component_id: &str) -> Option<&UIComponent> {
        self.components.get(component_id)
    }

    // Get component property
    pub fn get_component_property(&self, component_id: &str, property: &str) -> Option<&UIValue> {
        self.components.get(component_id)?
            .properties.get(property)
    }

    // Set component property
    pub fn set_component_property(&mut self, component_id: &str, property: String, value: UIValue) {
        if let Some(component) = self.components.get_mut(component_id) {
            component.properties.insert(property, value);
        }
    }

    // Queue an event
    pub fn queue_event(&mut self, event: UIEvent) {
        self.event_queue.push(event);
    }

    // Process queued events
    pub fn process_events(&mut self) -> Vec<UIEvent> {
        std::mem::take(&mut self.event_queue)
    }

    // Render the UI (platform-specific)
    pub fn render(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            self.render_web();
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.render_desktop();
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn render_web(&mut self) {
        if let Some(web_ctx) = &self.web_context {
            // Clear canvas
            let gl = &web_ctx.gl_context;
            gl.clear_color(0.1, 0.1, 0.1, 1.0);
            gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

            // Render components using WebGL
            for component in self.components.values() {
                if !component.visible {
                    continue;
                }

                self.render_web_component(component, gl);
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn render_web_component(&self, component: &UIComponent, gl: &WebGlRenderingContext) {
        match component.component_type {
            UIComponentType::Button => {
                // Render button as colored rectangle with text
                self.render_web_rectangle(component, gl);

                // In a real implementation, we'd render text using a text rendering library
                // For now, we'll just use colored rectangles
            }
            UIComponentType::Canvas => {
                // Main canvas area - usually handled by Bevy's rendering
                self.render_web_rectangle(component, gl);
            }
            UIComponentType::Panel | UIComponentType::Window => {
                self.render_web_rectangle(component, gl);
            }
            _ => {
                // Default rendering
                self.render_web_rectangle(component, gl);
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn render_web_rectangle(&self, component: &UIComponent, gl: &WebGlRenderingContext) {
        // Get background color
        let bg_color = component.properties.get("background_color")
            .and_then(|v| match v {
                UIValue::Color(c) => Some(*c),
                _ => None,
            })
            .unwrap_or(Color::rgb(0.2, 0.2, 0.2));

        // Set color
        gl.uniform4f(
            gl.get_uniform_location(gl.get_parameter(gl.CURRENT_PROGRAM).unwrap().as_ref(), "u_color").as_ref(),
            bg_color.r(),
            bg_color.g(),
            bg_color.b(),
            1.0
        );

        // Render quad at component position/size
        // (Detailed WebGL rendering code would go here)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn render_desktop(&mut self) {
        // egui rendering is handled by the EguiPlugin
        // This function would be called from a Bevy system that has access to EguiContext
    }

    // Handle input events (platform-specific)
    pub fn handle_input(&mut self, input: &Input<KeyCode>, mouse_input: &Input<MouseButton>, mouse_pos: Vec2) {
        #[cfg(target_arch = "wasm32")]
        {
            // Web input handling is done via event listeners
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            // Desktop input handling
            if input.just_pressed(KeyCode::Space) {
                self.queue_event(UIEvent::KeyPress {
                    key: KeyCode::Space,
                    modifiers: KeyModifiers::default(),
                });
            }

            if mouse_input.just_pressed(MouseButton::Left) {
                // Find component under mouse
                if let Some(component_id) = self.find_component_at_position(mouse_pos) {
                    self.queue_event(UIEvent::Click {
                        component_id,
                        button: MouseButton::Left,
                    });
                }
            }
        }
    }

    // Find component at position
    fn find_component_at_position(&self, position: Vec2) -> Option<String> {
        // Find topmost component that contains the position
        let mut found_component = None;
        let mut max_z = i32::MIN;

        for (id, component) in &self.components {
            if component.visible &&
               position.x >= component.position.x &&
               position.x <= component.position.x + component.size.x &&
               position.y >= component.position.y &&
               position.y <= component.position.y + component.size.y &&
               component.z_index > max_z {

                max_z = component.z_index;
                found_component = Some(id.clone());
            }
        }

        found_component
    }

    // Get all components
    pub fn get_components(&self) -> &HashMap<String, UIComponent> {
        &self.components
    }

    // Clear all components
    pub fn clear(&mut self) {
        self.components.clear();
        self.event_queue.clear();
        self.focused_component = None;
    }
}

// Global UI instance for WebAssembly
#[cfg(target_arch = "wasm32")]
static UNIFIED_UI: once_cell::sync::Lazy<Arc<Mutex<UnifiedUI>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(UnifiedUI::default())));

// WebAssembly exports
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl UnifiedUI {
    // JavaScript-callable functions
    #[wasm_bindgen]
    pub fn create_button(id: &str, x: f32, y: f32, width: f32, height: f32, text: &str) {
        let mut ui = UNIFIED_UI.lock().unwrap();
        ui.add_component(UIComponent {
            id: id.to_string(),
            component_type: UIComponentType::Button,
            position: Vec2::new(x, y),
            size: Vec2::new(width, height),
            properties: HashMap::from([
                ("text".to_string(), UIValue::String(text.to_string())),
            ]),
            children: vec![],
            parent: None,
            visible: true,
            enabled: true,
            z_index: 1,
        });
    }

    #[wasm_bindgen]
    pub fn set_component_visible(id: &str, visible: bool) {
        let mut ui = UNIFIED_UI.lock().unwrap();
        if let Some(component) = ui.components.get_mut(id) {
            component.visible = visible;
        }
    }

    #[wasm_bindgen]
    pub fn render_frame() {
        let mut ui = UNIFIED_UI.lock().unwrap();
        ui.render();
    }
}

// Bevy plugin for unified UI
pub struct UnifiedUIPlugin;

impl Plugin for UnifiedUIPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UnifiedUI::default())
           .add_systems(Startup, setup_unified_ui)
           .add_systems(Update, update_unified_ui);
    }
}

// Setup system
fn setup_unified_ui(mut ui: ResMut<UnifiedUI>) {
    ui.init(&mut App::empty()); // This is a simplified call
}

// Update system
fn update_unified_ui(
    mut ui: ResMut<UnifiedUI>,
    input: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
    windows: Query<&Window>,
) {
    // Get mouse position
    let mouse_pos = windows.single().cursor_position().unwrap_or(Vec2::ZERO);

    // Handle input
    ui.handle_input(&input, &mouse_input, mouse_pos);

    // Process events
    let events = ui.process_events();
    for event in events {
        match event {
            UIEvent::Click { component_id, .. } => {
                if component_id == "play_button" {
                    // Handle play button click
                    info!("Play button clicked");
                } else if component_id == "stop_button" {
                    // Handle stop button click
                    info!("Stop button clicked");
                }
            }
            UIEvent::KeyPress { key: KeyCode::Space, .. } => {
                // Handle spacebar (play/pause)
                info!("Spacebar pressed - toggle play/pause");
            }
            _ => {}
        }
    }

    // Render UI
    ui.render();
}

// Web logging helper
#[cfg(target_arch = "wasm32")]
fn web_log(message: &str) {
    console::log_1(&message.into());
}

#[cfg(not(target_arch = "wasm32"))]
fn web_log(_message: &str) {
    // No-op on desktop
}