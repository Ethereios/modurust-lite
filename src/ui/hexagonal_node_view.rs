use egui::{Color32, Pos2, Rect, Stroke, Vec2, Shape, CornerRadius, StrokeKind};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::audio_engine::bridge::{AudioEngineBridge, AudioParamMessage};

#[derive(Clone)]
pub struct HexNodeViewState {
    pub nodes: Vec<HexNode>,
    pub connections: Vec<NodeConnection>,
    pub next_node_id: usize,
    pub dragging_node_id: Option<usize>,
    pub drag_offset: Vec2,
    pub connecting_from: Option<(usize, usize, PortType)>,
    pub hovered_port: Option<(usize, usize, PortType)>,
    pub show_parameters: bool,
    pub grid_snap: bool,
    pub show_signal_levels: bool,
    pub lock_nodes: bool,
    pub sides: usize,
    pub ports_on_vertices: bool,
    pub show_debug_overlay: bool,
    pub audio_bridge: Option<Arc<Mutex<AudioEngineBridge>>>,
    pub selected_node_id: Option<usize>,
    pub node_params: HashMap<usize, HashMap<String, f32>>, // UI parameter cache
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PortType {
    Input,
    Output,
}

#[derive(Clone, Debug)]
pub struct NodePort {
    pub name: String,
    pub port_type: PortType,
    pub position: Pos2,
}

#[derive(Clone, Debug)]
pub struct NodeConnection {
    pub from_node: usize,
    pub from_port: usize,
    pub to_node: usize,
    pub to_port: usize,
}

#[derive(Clone, Debug)]
pub struct HexNode {
    pub id: usize,
    pub name: String,
    pub node_type: String,
    pub position: Pos2,
    pub input_ports: Vec<NodePort>,
    pub output_ports: Vec<NodePort>,
    pub width: f32,
    pub height: f32,
    pub sides: usize,
}

impl Default for HexNodeViewState {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            connections: Vec::new(),
            next_node_id: 0,
            dragging_node_id: None,
            drag_offset: Vec2::ZERO,
            connecting_from: None,
            hovered_port: None,
            show_parameters: true,
            grid_snap: false,
            show_signal_levels: false,
            lock_nodes: false,
            sides: 6,
            ports_on_vertices: true,
            show_debug_overlay: false,
            audio_bridge: None,
            selected_node_id: None,
            node_params: HashMap::new(),
        }
    }
}

impl HexNodeViewState {
    pub fn set_audio_bridge(&mut self, bridge: Arc<Mutex<AudioEngineBridge>>) {
        self.audio_bridge = Some(bridge);
    }

    pub fn apply_layout_settings(&mut self) {
        for node in &mut self.nodes {
            node.sides = self.sides;
            if self.ports_on_vertices {
                node.update_port_positions_on_vertices();
            } else {
                node.update_port_positions_on_edges();
            }
        }
    }

    pub fn add_node(&mut self, name: &str, node_type: &str, position: Pos2) {
        let mut node = HexNode::new(
            self.next_node_id,
            name.to_string(),
            node_type.to_string(),
            position,
        );
        node.sides = self.sides;
        
        // Add default ports based on node type
        if node_type.starts_with("generator") {
            node.output_ports.push(NodePort {
                name: "out".to_string(),
                port_type: PortType::Output,
                position: Pos2::ZERO, // Will be updated later
            });
        } else if node_type.starts_with("filter") || node_type.starts_with("effect") {
            node.input_ports.push(NodePort {
                name: "in".to_string(),
                port_type: PortType::Input,
                position: Pos2::ZERO, // Will be updated later
            });
            node.output_ports.push(NodePort {
                name: "out".to_string(),
                port_type: PortType::Output,
                position: Pos2::ZERO, // Will be updated later
            });
        } else {
            // Default node has both input and output
            node.input_ports.push(NodePort {
                name: "in".to_string(),
                port_type: PortType::Input,
                position: Pos2::ZERO,
            });
            node.output_ports.push(NodePort {
                name: "out".to_string(),
                port_type: PortType::Output,
                position: Pos2::ZERO,
            });
        }
        
        // Update port positions according to current policy
        if self.ports_on_vertices {
            node.update_port_positions_on_vertices();
        } else {
            node.update_port_positions_on_edges();
        }
        
        self.next_node_id += 1;
        self.nodes.push(node);
        let nid = self.next_node_id - 1;

        // Initialize default UI parameters based on node type
        let mut defaults: HashMap<String, f32> = HashMap::new();
        if node_type.starts_with("generator") {
            defaults.insert("frequency".to_string(), 440.0);
        } else if node_type.starts_with("filter") {
            defaults.insert("cutoff".to_string(), 1000.0);
            defaults.insert("resonance".to_string(), 0.2);
        } else if node_type.starts_with("effect.delay") {
            defaults.insert("delay_time".to_string(), 0.25);
            defaults.insert("feedback".to_string(), 0.3);
            defaults.insert("mix".to_string(), 0.5);
        } else if node_type.starts_with("effect.reverb") {
            defaults.insert("room_size".to_string(), 0.5);
            defaults.insert("damping".to_string(), 0.3);
            defaults.insert("mix".to_string(), 0.5);
        }
        self.node_params.insert(nid, defaults);

        // Notify audio engine about node creation (safe locking, no unwrap)
        if let Some(br) = &self.audio_bridge {
            if let Some(last) = self.nodes.last() {
                let ui_id = format!("node{}", last.id);
                match br.lock() {
                    Ok(bridge) => {
                        if let Err(e) = bridge.send_param(AudioParamMessage::CreateNode(last.node_type.clone(), ui_id)) {
                            eprintln!("UI→Audio send CreateNode failed: {}", e);
                        }
                    }
                    Err(poison) => {
                        eprintln!("Audio bridge mutex poisoned while creating node: {}", poison);
                    }
                }
            }
        }
    }

    #[allow(deprecated)]
    pub fn draw(&mut self, ui: &mut egui::Ui, rect: Rect) {
        // Draw background
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, Color32::from_rgb(20, 20, 30));
        
        // Draw grid
        let grid_color = Color32::from_rgba_premultiplied(60, 60, 80, 100);
        let grid_size = 40.0;
        
        // Vertical lines
        for x in (rect.left() as i32..rect.right() as i32).step_by(grid_size as usize) {
            painter.line_segment(
                [Pos2::new(x as f32, rect.top()), Pos2::new(x as f32, rect.bottom())],
                Stroke::new(1.0, grid_color),
            );
        }
        
        // Horizontal lines
        for y in (rect.top() as i32..rect.bottom() as i32).step_by(grid_size as usize) {
            painter.line_segment(
                [Pos2::new(rect.left(), y as f32), Pos2::new(rect.right(), y as f32)],
                Stroke::new(1.0, grid_color),
            );
        }
        
        // Draw connections first (behind nodes)
        for conn in &self.connections {
            if let (Some(from_node), Some(to_node)) = (
                self.nodes.iter().find(|n| n.id == conn.from_node),
                self.nodes.iter().find(|n| n.id == conn.to_node),
            ) {
                let from_port = from_node
                    .output_ports
                    .get(conn.from_port)
                    .map(|p| p.position)
                    .unwrap_or(from_node.get_rect().center());
                let to_port = to_node
                    .input_ports
                    .get(conn.to_port)
                    .map(|p| p.position)
                    .unwrap_or(to_node.get_rect().center());

                // Curved connection
                let cp1 = Pos2::new(from_port.x + 40.0, from_port.y);
                let cp2 = Pos2::new(to_port.x - 40.0, to_port.y);
                let steps = 20;
                let mut last = from_port;
                for i in 1..=steps {
                    let t = i as f32 / steps as f32;
                    let x = (1.0 - t).powi(3) * from_port.x
                        + 3.0 * (1.0 - t).powi(2) * t * cp1.x
                        + 3.0 * (1.0 - t) * t.powi(2) * cp2.x
                        + t.powi(3) * to_port.x;
                    let y = (1.0 - t).powi(3) * from_port.y
                        + 3.0 * (1.0 - t).powi(2) * t * cp1.y
                        + 3.0 * (1.0 - t) * t.powi(2) * cp2.y
                        + t.powi(3) * to_port.y;
                    let pt = Pos2::new(x, y);
                    painter.line_segment([last, pt], Stroke::new(3.0, Color32::from_rgb(160, 160, 220)));
                    last = pt;
                }
            }
        }

        // Draw nodes
        for node in &self.nodes {
            let node_rect = node.get_rect();
            let node_color = self.get_node_color(&node.node_type);
            // Draw hexagon shape instead of rectangle
            let verts = node.hex_polygon_points();
            painter.add(Shape::convex_polygon(verts.clone(), node_color, Stroke::new(2.0, Color32::WHITE)));
            
            // Draw node title
            painter.text(
                Pos2::new(node_rect.center().x, node_rect.top() + 12.0),
                egui::Align2::CENTER_CENTER,
                &node.name,
                egui::FontId::proportional(16.0),
                Color32::WHITE,
            );
            
            // Draw node type
            painter.text(
                Pos2::new(node_rect.center().x, node_rect.top() + 30.0),
                egui::Align2::CENTER_CENTER,
                &node.node_type,
                egui::FontId::proportional(12.0),
                Color32::LIGHT_GRAY,
            );

            // Draw input ports (left side)
            for (i, port) in node.input_ports.iter().enumerate() {
                let radius = 7.0;
                let color = if self.hovered_port == Some((node.id, i, PortType::Input)) {
                    Color32::from_rgb(120, 220, 120)
                } else {
                    Color32::from_rgb(80, 180, 80)
                };
                painter.circle_filled(port.position, radius, color);
                painter.circle_stroke(port.position, radius, Stroke::new(2.0, Color32::WHITE));
                painter.text(
                    Pos2::new(port.position.x + 10.0, port.position.y),
                    egui::Align2::LEFT_CENTER,
                    &port.name,
                    egui::FontId::proportional(10.0),
                    Color32::LIGHT_GRAY,
                );
            }

            // Draw output ports (right side)
            for (i, port) in node.output_ports.iter().enumerate() {
                let radius = 7.0;
                let color = if self.hovered_port == Some((node.id, i, PortType::Output)) {
                    Color32::from_rgb(220, 120, 120)
                } else {
                    Color32::from_rgb(180, 80, 80)
                };
                painter.circle_filled(port.position, radius, color);
                painter.circle_stroke(port.position, radius, Stroke::new(2.0, Color32::WHITE));
                painter.text(
                    Pos2::new(port.position.x - 10.0, port.position.y),
                    egui::Align2::RIGHT_CENTER,
                    &port.name,
                    egui::FontId::proportional(10.0),
                    Color32::LIGHT_GRAY,
                );
            }
            
            // Debug overlay markers on vertices
            if self.show_debug_overlay {
                for v in &verts {
                    painter.circle_stroke(*v, 4.0, Stroke::new(1.5, Color32::YELLOW));
                }
            }
        }

        // Global debug overlay information
        if self.show_debug_overlay {
            let info = format!(
                "dragging_node_id={:?}\nconnecting_from={:?}\nhovered_port={:?}\nsides={} ports_on_vertices={}",
                self.dragging_node_id, self.connecting_from, self.hovered_port, self.sides, self.ports_on_vertices
            );
            painter.text(
                Pos2::new(rect.left() + 10.0, rect.top() + 10.0),
                egui::Align2::LEFT_TOP,
                info,
                egui::FontId::monospace(12.0),
                Color32::LIGHT_GRAY,
            );
        }

        // Connection preview while wiring
        if let Some((node_id, port_idx, port_ty)) = self.connecting_from {
            if let Some(mouse) = ui.input(|i| i.pointer.hover_pos()) {
                if let Some(node) = self.nodes.iter().find(|n| n.id == node_id) {
                    let start = match port_ty {
                        PortType::Input => node.input_ports.get(port_idx).map(|p| p.position),
                        PortType::Output => node.output_ports.get(port_idx).map(|p| p.position),
                    };
                    if let Some(start) = start {
                        painter.line_segment([start, mouse], Stroke::new(2.0, Color32::from_rgb(200, 200, 200)));
                    }
                }
            }
        }

        // Handle interactions
        self.handle_interactions(ui, rect);

        // Parameter panel for selected node
        if self.show_parameters {
            if let Some(sel_id) = self.selected_node_id {
                if let Some(node) = self.nodes.iter().find(|n| n.id == sel_id) {
                    let panel_rect = Rect::from_min_max(
                        Pos2::new(rect.left() + 8.0, rect.bottom() - 140.0),
                        Pos2::new(rect.left() + 280.0, rect.bottom() - 8.0),
                    );
                    // Background
                    let painter = ui.painter();
                    painter.rect_filled(panel_rect, 6.0, Color32::from_rgb(30, 30, 40));
                    painter.rect_stroke(panel_rect, CornerRadius::same(6_u8), Stroke::new(1.0, Color32::from_rgb(80, 80, 100)), StrokeKind::Inside);

                    // Build UI within rect
                    ui.allocate_ui_at_rect(panel_rect, |ui| {
                        ui.label(format!("Parameters: {}", node.name));
                        let pid = node.id;
                        let params = self.node_params.entry(pid).or_insert_with(HashMap::new);

                        // Helper to send param (non-panicking lock with logging)
                        let send_param = |key: &str, val: f32| {
                            if let Some(br) = &self.audio_bridge {
                                let ui_id = format!("node{}", pid);
                                match br.lock() {
                                    Ok(bridge) => {
                                        if let Err(e) = bridge.send_param(AudioParamMessage::SetNodeParameter(ui_id, key.to_string(), val)) {
                                            eprintln!("UI→Audio send_param failed: {}", e);
                                        }
                                    }
                                    Err(poison) => {
                                        eprintln!("Audio bridge mutex poisoned while sending param: {}", poison);
                                    }
                                }
                            } else {
                                eprintln!("Audio bridge not available; cannot send node parameter {}", key);
                            }
                        };

                        match node.node_type.as_str() {
                            t if t.starts_with("generator") => {
                                let v = params.entry("frequency".to_string()).or_insert(440.0);
                                if ui.add(egui::Slider::new(v, 20.0..=20000.0).logarithmic(true).text("Freq (Hz)")).changed() {
                                    send_param("frequency", *v);
                                }
                            }
                            t if t.starts_with("filter") => {
                                let cutoff = params.entry("cutoff".to_string()).or_insert(1000.0);
                                if ui.add(egui::Slider::new(cutoff, 20.0..=20000.0).logarithmic(true).text("Cutoff (Hz)")).changed() {
                                    send_param("cutoff", *cutoff);
                                }
                                let res = params.entry("resonance".to_string()).or_insert(0.2);
                                if ui.add(egui::Slider::new(res, 0.0..=1.0).text("Resonance")).changed() {
                                    send_param("resonance", *res);
                                }
                            }
                            t if t.starts_with("effect.delay") => {
                                let dt = params.entry("delay_time".to_string()).or_insert(0.25);
                                if ui.add(egui::Slider::new(dt, 0.01..=2.0).text("Delay (s)")).changed() {
                                    send_param("delay_time", *dt);
                                }
                                let fb = params.entry("feedback".to_string()).or_insert(0.3);
                                if ui.add(egui::Slider::new(fb, 0.0..=0.95).text("Feedback")).changed() {
                                    send_param("feedback", *fb);
                                }
                                let mix = params.entry("mix".to_string()).or_insert(0.5);
                                if ui.add(egui::Slider::new(mix, 0.0..=1.0).text("Mix")).changed() {
                                    send_param("mix", *mix);
                                }
                            }
                            t if t.starts_with("effect.reverb") => {
                                let room = params.entry("room_size".to_string()).or_insert(0.5);
                                if ui.add(egui::Slider::new(room, 0.1..=10.0).text("Room Size")).changed() {
                                    send_param("room_size", *room);
                                }
                                let damp = params.entry("damping".to_string()).or_insert(0.3);
                                if ui.add(egui::Slider::new(damp, 0.0..=1.0).text("Damping")).changed() {
                                    send_param("damping", *damp);
                                }
                                let mix = params.entry("mix".to_string()).or_insert(0.5);
                                if ui.add(egui::Slider::new(mix, 0.0..=1.0).text("Mix")).changed() {
                                    send_param("mix", *mix);
                                }
                            }
                            _ => {
                                ui.label("No parameters for this node type");
                            }
                        }
                    });
                }
            }
        }
    }
    
    fn get_node_color(&self, node_type: &str) -> Color32 {
        match node_type {
            t if t.starts_with("generator") => Color32::from_rgb(60, 120, 200),
            t if t.starts_with("filter") => Color32::from_rgb(120, 60, 200),
            t if t.starts_with("effect") => Color32::from_rgb(200, 120, 60),
            _ => Color32::from_rgb(100, 100, 100),
        }
    }
    
    fn handle_interactions(&mut self, ui: &mut egui::Ui, _rect: Rect) {
        let hover_pos = ui.input(|i| i.pointer.hover_pos());
        let is_mouse_down = ui.input(|i| i.pointer.primary_down());
        let is_mouse_clicked = ui.input(|i| i.pointer.primary_clicked());
        let is_mouse_released = ui.input(|i| i.pointer.primary_released());
        
        // Handle drop
        if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
            if ui.input(|i| i.pointer.any_released()) {
                if let Some(payload) = ui.memory(|mem| mem.data.get_temp::<String>(ui.id())) {
                    println!("Dropped file: {}", payload);
                    self.add_node("Sample", "generator.sample", hover_pos);
                }
            }
        }

        // Port hover detection
        if let Some(pos) = hover_pos {
            self.hovered_port = self.port_hit_test(pos);
        } else {
            self.hovered_port = None;
        }

        // Wiring interaction
        if let Some(pos) = hover_pos {
            if is_mouse_clicked {
                if let Some((nid, pidx, pty)) = self.port_hit_test(pos) {
                    // Begin connection from this port
                    self.connecting_from = Some((nid, pidx, pty));
                    // Do not change selection when starting a wire from a port
                } else {
                    // Otherwise, possibly start dragging a node
                    if !self.lock_nodes {
                        for node in &self.nodes {
                            if node.contains(pos) {
                                self.dragging_node_id = Some(node.id);
                                self.drag_offset = node.position - pos;
                                self.selected_node_id = Some(node.id);
                                break;
                            }
                        }
                        // If click not on any node, clear selection
                        if self.dragging_node_id.is_none() {
                            self.selected_node_id = None;
                        }
                    }
                }
            }
        }

        // Handle node dragging
        if let Some(pos) = hover_pos {
            // Update node position while dragging
            if is_mouse_down && self.dragging_node_id.is_some() {
                if let Some(node_id) = self.dragging_node_id {
                    if let Some(node) = self.nodes.iter_mut().find(|n| n.id == node_id) {
                        let mut new_pos = pos + self.drag_offset;
                        if self.grid_snap {
                            new_pos = snap_to_hex(new_pos);
                        }
                        node.position = new_pos;
                        node.update_port_positions();
                    }
                }
            }
            
            // Stop dragging
            if is_mouse_released {
                self.dragging_node_id = None;
                // Complete wiring if in progress
                if let Some((from_nid, from_pidx, from_ty)) = self.connecting_from.take() {
                    if let Some((to_nid, to_pidx, to_ty)) = self.hovered_port {
                        // Must be opposite types and different node
                        if matches!((from_ty, to_ty), (PortType::Output, PortType::Input) | (PortType::Input, PortType::Output))
                            && from_nid != to_nid
                        {
                            // Normalize to Output -> Input connection
                            let (src_nid, src_pidx, dst_nid, dst_pidx) = match from_ty {
                                PortType::Output => (from_nid, from_pidx, to_nid, to_pidx),
                                PortType::Input => (to_nid, to_pidx, from_nid, from_pidx),
                            };

                            // Only add if indices exist
                            let src_ok = self
                                .nodes
                                .iter()
                                .find(|n| n.id == src_nid)
                                .and_then(|n| n.output_ports.get(src_pidx))
                                .is_some();
                            let dst_ok = self
                                .nodes
                                .iter()
                                .find(|n| n.id == dst_nid)
                                .and_then(|n| n.input_ports.get(dst_pidx))
                                .is_some();

                            if src_ok && dst_ok {
                                self.connections.push(NodeConnection {
                                    from_node: src_nid,
                                    from_port: src_pidx,
                                    to_node: dst_nid,
                                    to_port: dst_pidx,
                                });

                                // Notify audio engine about node connection
                                if let Some(br) = &self.audio_bridge {
                                    let src_node = self.nodes.iter().find(|n| n.id == src_nid);
                                    let dst_node = self.nodes.iter().find(|n| n.id == dst_nid);
                                    if let (Some(src), Some(dst)) = (src_node, dst_node) {
                                        let from_ui = format!("node{}", src.id);
                                        let to_ui = format!("node{}", dst.id);
                                        let from_port = src.output_ports.get(src_pidx).map(|p| p.name.clone()).unwrap_or_else(|| "out".to_string());
                                        let to_port = dst.input_ports.get(dst_pidx).map(|p| p.name.clone()).unwrap_or_else(|| "in".to_string());
                                        match br.lock() {
                                            Ok(bridge) => {
                                                if let Err(e) = bridge.send_param(AudioParamMessage::ConnectNodes(from_ui, to_ui, from_port, to_port)) {
                                                    eprintln!("UI→Audio send ConnectNodes failed: {}", e);
                                                }
                                            }
                                            Err(poison) => {
                                                eprintln!("Audio bridge mutex poisoned while connecting nodes: {}", poison);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn port_hit_test(&self, pos: Pos2) -> Option<(usize, usize, PortType)> {
        let radius = 10.0;
        // Check inputs
        for node in &self.nodes {
            for (i, port) in node.input_ports.iter().enumerate() {
                if (port.position - pos).length() <= radius {
                    return Some((node.id, i, PortType::Input));
                }
            }
            for (i, port) in node.output_ports.iter().enumerate() {
                if (port.position - pos).length() <= radius {
                    return Some((node.id, i, PortType::Output));
                }
            }
        }
        None
    }
}

impl HexNode {
    pub fn new(id: usize, name: String, node_type: String, position: Pos2) -> Self {
        Self {
            id,
            name,
            node_type,
            position,
            input_ports: Vec::new(),
            output_ports: Vec::new(),
            width: 120.0,
            height: 100.0,
            sides: 6,
        }
    }
    
    pub fn get_rect(&self) -> Rect {
        Rect::from_min_size(self.position, Vec2::new(self.width, self.height))
    }
    
    pub fn contains(&self, pos: Pos2) -> bool {
        let verts = self.hex_polygon_points();
        point_in_polygon(pos, &verts)
    }
    
    pub fn update_port_positions(&mut self) {
        let verts = self.hex_polygon_points();
        // Order reference for hex: [top, top-right, bottom-right, bottom, bottom-left, top-left]
        // For generic polygons, fallback to distributing along two edges near left/right
        let left_start = verts[(self.sides + self.sides - 1) % self.sides];
        let left_end = verts[(self.sides + self.sides - 2) % self.sides];
        let right_start = verts[1 % self.sides];
        let right_end = verts[2 % self.sides];

        let n_in = self.input_ports.len().max(1);
        for (i, port) in self.input_ports.iter_mut().enumerate() {
            let t = (i as f32 + 1.0) / (n_in as f32 + 1.0);
            port.position = Pos2::new(
                left_start.x + t * (left_end.x - left_start.x),
                left_start.y + t * (left_end.y - left_start.y),
            );
        }

        let n_out = self.output_ports.len().max(1);
        for (i, port) in self.output_ports.iter_mut().enumerate() {
            let t = (i as f32 + 1.0) / (n_out as f32 + 1.0);
            port.position = Pos2::new(
                right_start.x + t * (right_end.x - right_start.x),
                right_start.y + t * (right_end.y - right_start.y),
            );
        }
    }

    pub fn update_port_positions_on_edges(&mut self) {
        // Keep existing behavior of distributing ports along left/right edges
        self.update_port_positions();
    }

    pub fn update_port_positions_on_vertices(&mut self) {
        let verts = self.hex_polygon_points();
        // Assign inputs to even-indexed vertices, outputs to odd-indexed
        let mut in_indices: Vec<usize> = (0..self.sides).filter(|i| i % 2 == 0).collect();
        let mut out_indices: Vec<usize> = (0..self.sides).filter(|i| i % 2 == 1).collect();
        if in_indices.is_empty() { in_indices.push(0); }
        if out_indices.is_empty() { out_indices.push(0); }

        for (i, port) in self.input_ports.iter_mut().enumerate() {
            let idx = in_indices[i % in_indices.len()];
            port.position = verts[idx];
        }
        for (i, port) in self.output_ports.iter_mut().enumerate() {
            let idx = out_indices[i % out_indices.len()];
            port.position = verts[idx];
        }
    }

    pub fn hex_polygon_points(&self) -> Vec<Pos2> {
        // Pointy-top regular polygon centered within bounding rect
        let rect = self.get_rect();
        let cx = rect.center().x;
        let cy = rect.center().y;
        let radius = (self.width.min(self.height) * 0.5) - 4.0;
        let mut points = Vec::with_capacity(self.sides);
        let step = 2.0 * std::f32::consts::PI / self.sides as f32;
        for k in 0..self.sides {
            let angle = -std::f32::consts::FRAC_PI_2 + k as f32 * step;
            let x = cx + radius * angle.cos();
            let y = cy + radius * angle.sin();
            points.push(Pos2::new(x, y));
        }
        points
    }
}

fn point_in_polygon(p: Pos2, verts: &[Pos2]) -> bool {
    // Ray casting algorithm for convex polygon
    let mut inside = false;
    let mut j = verts.len() - 1;
    for i in 0..verts.len() {
        let xi = verts[i].x;
        let yi = verts[i].y;
        let xj = verts[j].x;
        let yj = verts[j].y;
        let intersect = ((yi > p.y) != (yj > p.y))
            && (p.x < (xj - xi) * (p.y - yi) / (yj - yi + 1e-6) + xi);
        if intersect { inside = !inside; }
        j = i;
    }
    inside
}

fn snap_to_hex(pos: Pos2) -> Pos2 {
    // Snap to nearest pointy-top hex center using axial coordinates
    let size = 40.0; // hex radius
    let q = (pos.x * (3.0f32).sqrt() / 3.0 - pos.y / 3.0) / size;
    let r = (pos.y * 2.0 / 3.0) / size;
    let (rq, rr) = axial_round(q, r);
    axial_to_pixel(rq, rr, size)
}

#[allow(unused_assignments)]
fn axial_round(q: f32, r: f32) -> (i32, i32) {
    // Convert axial to cube, round, then back
    let x = q;
    let z = r;
    let y = -x - z;
    let mut rx = x.round();
    let mut ry = y.round();
    let mut rz = z.round();

    let x_diff = (rx - x).abs();
    let y_diff = (ry - y).abs();
    let z_diff = (rz - z).abs();

    if x_diff > y_diff && x_diff > z_diff {
        rx = -ry - rz;
    } else if y_diff > z_diff {
        ry = -rx - rz;
    } else {
        rz = -rx - ry;
    }
    (rx as i32, rz as i32)
}

fn axial_to_pixel(q: i32, r: i32, size: f32) -> Pos2 {
    // Pointy-top axial to pixel conversion
    let x = size * (3.0f32).sqrt() * (q as f32 + r as f32 / 2.0);
    let y = size * 1.5 * r as f32;
    Pos2::new(x, y)
}
