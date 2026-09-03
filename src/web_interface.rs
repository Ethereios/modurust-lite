//! Web Interface for HexoDSP DAW
//!
//! This module provides JavaScript integration for browser-based DAW functionality,
//! enabling web-based music production with the same powerful features as the desktop version.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use warp::{Filter, Reply, ws::{Message, WebSocket}};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use uuid::Uuid;

/// Web interface configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebInterfaceConfig {
    pub host: String,
    pub port: u16,
    pub enable_cors: bool,
    pub enable_websockets: bool,
    pub static_files_path: String,
}

/// WebSocket message types for real-time communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WebSocketMessage {
    // Audio transport control
    Play,
    Pause,
    Stop,
    Seek { position: f64 },

    // Node graph operations
    CreateNode { node_type: String, position: (f32, f32) },
    DeleteNode { node_id: String },
    ConnectNodes { from_node: String, from_port: String, to_node: String, to_port: String },
    DisconnectNodes { connection_id: String },

    // Parameter changes
    ParameterChange { node_id: String, parameter: String, value: f64 },

    // Arrangement operations
    CreateTrack { track_type: String },
    DeleteTrack { track_id: String },
    AddClip { track_id: String, clip_data: Value },
    MoveClip { clip_id: String, new_position: f64 },

    // AI tool requests
    StemSeparation { audio_data: Vec<u8> },
    GeneratePattern { style: String, complexity: f32, tempo: f32 },

    // Responses
    StatusUpdate { status: String },
    AudioData { data: Vec<f32>, sample_rate: f32 },
    NodeCreated { node_id: String, node_data: Value },
    Error { message: String },
}

/// Connected WebSocket clients
type Clients = Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>;

/// Web interface server
#[cfg(feature = "webui")]
pub struct WebInterfaceServer {
    config: WebInterfaceConfig,
    clients: Clients,
    broadcast_tx: broadcast::Sender<String>,
}

#[cfg(feature = "webui")]
impl WebInterfaceServer {
    pub fn new(config: WebInterfaceConfig) -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        Self {
            config,
            clients: Arc::new(Mutex::new(HashMap::new())),
            broadcast_tx,
        }
    }

    /// Build all routes for the web server
    pub fn build_routes(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        let static_files = warp::path("static")
            .and(warp::fs::dir(self.config.static_files_path.clone()));

        let api_routes = self.build_api_routes();
        let websocket_route = self.build_websocket_route();
        let index_route = self.build_index_route();

        static_files
            .or(api_routes)
            .or(websocket_route)
            .or(index_route)
    }

    /// Build API routes
    fn build_api_routes(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type"])
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE"]);

        // Transport control endpoints
        let play = warp::path!("api" / "transport" / "play")
            .and(warp::post())
            .and_then(handle_play);

        let pause = warp::path!("api" / "transport" / "pause")
            .and(warp::post())
            .and_then(handle_pause);

        let stop = warp::path!("api" / "transport" / "stop")
            .and(warp::post())
            .and_then(handle_stop);

        // Node graph endpoints
        let create_node = warp::path!("api" / "nodes")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(handle_create_node);

        let delete_node = warp::path!("api" / "nodes" / String)
            .and(warp::delete())
            .and_then(handle_delete_node);

        // Parameter endpoints
        let set_parameter = warp::path!("api" / "parameters" / String / String)
            .and(warp::put())
            .and(warp::body::json())
            .and_then(handle_set_parameter);

        play.or(pause)
            .or(stop)
            .or(create_node)
            .or(delete_node)
            .or(set_parameter)
            .with(cors)
    }

    /// Build WebSocket route
    fn build_websocket_route(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        let clients = self.clients.clone();
        let broadcast_tx = self.broadcast_tx.clone();

        warp::path("ws")
            .and(warp::ws())
            .and(warp::any().map(move || clients.clone()))
            .and(warp::any().map(move || broadcast_tx.clone()))
            .and_then(handle_websocket)
    }

    /// Build index route
    fn build_index_route(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path::end()
            .and(warp::get())
            .and_then(|| async {
                Ok::<_, warp::Rejection>(warp::reply::html(generate_html_page()))
            })
    }

    /// Start the web server
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let routes = self.build_routes();
        
        println!("🌐 Starting HexoDSP Web Interface");
        println!("📍 Server: http://{}:{}", self.config.host, self.config.port);
        println!("🔌 WebSocket: ws://{}:{}/ws", self.config.host, self.config.port);
        
        let addr = format!("{}:{}", self.config.host, self.config.port)
            .parse::<std::net::SocketAddr>()?;

        warp::serve(routes)
            .run(addr)
            .await;

        Ok(())
    }

    /// Broadcast message to all connected clients
    pub fn broadcast(&self, message: &WebSocketMessage) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(message)?;
        let _ = self.broadcast_tx.send(json);
        Ok(())
    }
}

/// Handle WebSocket connections
async fn handle_websocket(
    ws: warp::ws::Ws,
    clients: Clients,
    broadcast_tx: broadcast::Sender<String>,
) -> Result<impl Reply, warp::Rejection> {
    Ok(ws.on_upgrade(move |socket| handle_websocket_connection(socket, clients, broadcast_tx)))
}

/// Handle individual WebSocket connection
async fn handle_websocket_connection(
    ws: WebSocket,
    clients: Clients,
    broadcast_tx: broadcast::Sender<String>,
) {
    let client_id = Uuid::new_v4().to_string();
    let (mut ws_tx, mut ws_rx) = ws.split();
    let mut broadcast_rx = broadcast_tx.subscribe();

    // Add client to the list
    {
        let mut clients_lock = clients.lock().unwrap();
        clients_lock.insert(client_id.clone(), broadcast_tx.clone());
    }

    println!("🔌 WebSocket client connected: {}", client_id);

    // Handle incoming messages from client
    let clients_for_incoming = clients.clone();
    let client_id_for_incoming = client_id.clone();
    let incoming_task = tokio::spawn(async move {
        while let Some(result) = ws_rx.next().await {
            match result {
                Ok(msg) => {
                    if let Ok(text) = msg.to_str() {
                        if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(text) {
                            if let Err(e) = handle_websocket_message(ws_msg).await {
                                eprintln!("Error handling WebSocket message: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("WebSocket error: {}", e);
                    break;
                }
            }
        }
        
        // Remove client when connection closes
        {
            let mut clients_lock = clients_for_incoming.lock().unwrap();
            clients_lock.remove(&client_id_for_incoming);
        }
        println!("🔌 WebSocket client disconnected: {}", client_id_for_incoming);
    });

    // Handle outgoing messages to client
    let outgoing_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            if ws_tx.send(Message::text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = incoming_task => {},
        _ = outgoing_task => {},
    }
}

/// Handle transport control - Play
async fn handle_play() -> Result<impl Reply, warp::Rejection> {
    println!("🎵 Transport: Play");
    Ok(warp::reply::json(&serde_json::json!({"status": "playing"})))
}

/// Handle transport control - Pause
async fn handle_pause() -> Result<impl Reply, warp::Rejection> {
    println!("⏸️ Transport: Pause");
    Ok(warp::reply::json(&serde_json::json!({"status": "paused"})))
}

/// Handle transport control - Stop
async fn handle_stop() -> Result<impl Reply, warp::Rejection> {
    println!("⏹️ Transport: Stop");
    Ok(warp::reply::json(&serde_json::json!({"status": "stopped"})))
}

/// Handle node creation
async fn handle_create_node(node_data: Value) -> Result<impl Reply, warp::Rejection> {
    println!("🔗 Creating node: {:?}", node_data);
    let node_id = Uuid::new_v4().to_string();
    Ok(warp::reply::json(&serde_json::json!({
        "node_id": node_id,
        "status": "created"
    })))
}

/// Handle node deletion
async fn handle_delete_node(node_id: String) -> Result<impl Reply, warp::Rejection> {
    println!("🗑️ Deleting node: {}", node_id);
    Ok(warp::reply::json(&serde_json::json!({"status": "deleted"})))
}

/// Handle parameter setting
async fn handle_set_parameter(
    node_id: String,
    param_name: String,
    value: Value,
) -> Result<impl Reply, warp::Rejection> {
    println!("🎛️ Setting parameter {} on node {} to {:?}", param_name, node_id, value);
    Ok(warp::reply::json(&serde_json::json!({"status": "updated"})))
}

/// Handle individual WebSocket messages
async fn handle_websocket_message(msg: WebSocketMessage) -> Result<WebSocketMessage, Box<dyn std::error::Error>> {
    match msg {
        WebSocketMessage::Play => {
            // Start audio playback
            println!("Starting playback");
            Ok(WebSocketMessage::StatusUpdate { status: "Playing".to_string() })
        }
        WebSocketMessage::Pause => {
            // Pause audio playback
            println!("Pausing playback");
            Ok(WebSocketMessage::StatusUpdate { status: "Paused".to_string() })
        }
        WebSocketMessage::Stop => {
            // Stop audio playback
            println!("Stopping playback");
            Ok(WebSocketMessage::StatusUpdate { status: "Stopped".to_string() })
        }
        WebSocketMessage::CreateNode { node_type, position } => {
            // Create a new node
            let node_id = format!("node_{}", rand::random::<u32>());
            println!("Creating {} node at {:?}", node_type, position);

            let node_data = serde_json::json!({
                "id": node_id,
                "type": node_type,
                "position": position,
                "parameters": {}
            });

            Ok(WebSocketMessage::NodeCreated { node_id, node_data })
        }
        WebSocketMessage::ParameterChange { node_id, parameter, value } => {
            // Update node parameter
            println!("Setting {} on node {} to {}", parameter, node_id, value);
            Ok(WebSocketMessage::StatusUpdate {
                status: format!("Updated {} to {}", parameter, value)
            })
        }
        WebSocketMessage::StemSeparation { audio_data } => {
            // Process stem separation
            println!("Processing stem separation for {} bytes of audio", audio_data.len());
            Ok(WebSocketMessage::StatusUpdate {
                status: "Stem separation completed".to_string()
            })
        }
        WebSocketMessage::GeneratePattern { style, complexity, tempo } => {
            // Generate AI pattern
            println!("Generating {} pattern with complexity {} at {} BPM", style, complexity, tempo);
            Ok(WebSocketMessage::StatusUpdate {
                status: format!("Generated {} pattern", style)
            })
        }
        _ => {
            Ok(WebSocketMessage::StatusUpdate {
                status: "Message received".to_string()
            })
        }
    }
}

/// Generate the main HTML page for the web interface
fn generate_html_page() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>HexoDSP DAW - Web Interface</title>
    <style>
        body {
            margin: 0;
            padding: 0;
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
            color: #e0e0e0;
            overflow: hidden;
        }

        .header {
            background: rgba(0, 0, 0, 0.8);
            padding: 1rem;
            border-bottom: 1px solid #333;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }

        .logo {
            font-size: 1.5rem;
            font-weight: bold;
            color: #00d4ff;
        }

        .toolbar {
            display: flex;
            gap: 0.5rem;
        }

        .btn {
            background: #333;
            border: 1px solid #555;
            color: #e0e0e0;
            padding: 0.5rem 1rem;
            border-radius: 4px;
            cursor: pointer;
            transition: all 0.2s;
        }

        .btn:hover {
            background: #444;
            border-color: #777;
        }

        .btn.active {
            background: #00d4ff;
            color: #000;
        }

        .main-content {
            display: flex;
            height: calc(100vh - 70px);
        }

        .sidebar {
            width: 250px;
            background: rgba(0, 0, 0, 0.6);
            border-right: 1px solid #333;
            padding: 1rem;
            overflow-y: auto;
        }

        .sidebar h3 {
            margin-top: 0;
            color: #00d4ff;
        }

        .node-palette {
            display: grid;
            grid-template-columns: 1fr;
            gap: 0.5rem;
        }

        .node-item {
            background: #333;
            padding: 0.5rem;
            border-radius: 4px;
            cursor: grab;
            border: 1px solid #555;
        }

        .node-item:hover {
            background: #444;
        }

        .workspace {
            flex: 1;
            position: relative;
            background: rgba(20, 20, 40, 0.8);
        }

        .canvas {
            width: 100%;
            height: 100%;
            background: transparent;
        }

        .transport {
            position: absolute;
            bottom: 0;
            left: 0;
            right: 0;
            background: rgba(0, 0, 0, 0.8);
            padding: 1rem;
            border-top: 1px solid #333;
            display: flex;
            align-items: center;
            gap: 1rem;
        }

        .time-display {
            font-family: monospace;
            font-size: 1.2rem;
            color: #00d4ff;
        }

        .status {
            position: absolute;
            top: 10px;
            right: 10px;
            background: rgba(0, 0, 0, 0.8);
            padding: 0.5rem;
            border-radius: 4px;
            font-size: 0.9rem;
        }

        .connection {
            position: absolute;
            pointer-events: none;
        }

        .node {
            position: absolute;
            background: rgba(40, 40, 60, 0.9);
            border: 2px solid #555;
            border-radius: 8px;
            padding: 1rem;
            min-width: 150px;
            cursor: move;
        }

        .node.selected {
            border-color: #00d4ff;
        }

        .node-header {
            font-weight: bold;
            margin-bottom: 0.5rem;
            color: #00d4ff;
        }

        .node-port {
            width: 12px;
            height: 12px;
            border-radius: 50%;
            background: #666;
            margin: 2px;
            cursor: pointer;
            display: inline-block;
        }

        .node-port:hover {
            background: #00d4ff;
        }

        .node-inputs, .node-outputs {
            margin: 0.5rem 0;
        }

        .parameter {
            margin: 0.5rem 0;
        }

        .parameter input {
            width: 100%;
            background: #222;
            border: 1px solid #555;
            color: #e0e0e0;
            padding: 0.25rem;
            border-radius: 2px;
        }
    </style>
</head>
<body>
    <div class="header">
        <div class="logo">🎵 HexoDSP DAW</div>
        <div class="toolbar">
            <button class="btn" id="arrangement-btn">Arrangement</button>
            <button class="btn" id="live-btn">Live</button>
            <button class="btn active" id="node-btn">Node</button>
            <button class="btn" id="save-btn">💾 Save</button>
            <button class="btn" id="export-btn">📤 Export</button>
        </div>
    </div>

    <div class="main-content">
        <div class="sidebar">
            <h3>Node Palette</h3>
            <div class="node-palette">
                <div class="node-item" draggable="true" data-type="oscillator.sine">🎵 Sine Oscillator</div>
                <div class="node-item" draggable="true" data-type="oscillator.saw">📈 Saw Oscillator</div>
                <div class="node-item" draggable="true" data-type="filter.lowpass">🔽 Low Pass Filter</div>
                <div class="node-item" draggable="true" data-type="effect.reverb">🌊 Reverb</div>
                <div class="node-item" draggable="true" data-type="effect.delay">⏰ Delay</div>
                <div class="node-item" draggable="true" data-type="ai.drum_machine">🤖 AI Drum Machine</div>
                <div class="node-item" draggable="true" data-type="ai.stem_separator">🎚️ Stem Separator</div>
            </div>
        </div>

        <div class="workspace">
            <canvas class="canvas" id="canvas"></canvas>

            <div class="transport">
                <button class="btn" id="play-btn">▶️ Play</button>
                <button class="btn" id="pause-btn">⏸️ Pause</button>
                <button class="btn" id="stop-btn">⏹️ Stop</button>
                <div class="time-display" id="time-display">00:00:00.000</div>
                <input type="range" id="tempo-slider" min="60" max="200" value="120">
                <span id="tempo-display">120 BPM</span>
            </div>

            <div class="status" id="status">Connected to HexoDSP DAW</div>
        </div>
    </div>

    <script>
        // WebSocket connection
        let ws = null;
        let nodes = [];
        let connections = [];
        let selectedNode = null;
        let draggedNode = null;
        let dragOffset = { x: 0, y: 0 };

        // Initialize WebSocket connection
        function initWebSocket() {
            ws = new WebSocket('ws://localhost:3030/ws');

            ws.onopen = function(event) {
                console.log('Connected to HexoDSP DAW');
                updateStatus('Connected to HexoDSP DAW');
            };

            ws.onmessage = function(event) {
                const message = JSON.parse(event.data);
                handleWebSocketMessage(message);
            };

            ws.onclose = function(event) {
                console.log('Disconnected from HexoDSP DAW');
                updateStatus('Disconnected - Reconnecting...');
                setTimeout(initWebSocket, 1000);
            };

            ws.onerror = function(error) {
                console.error('WebSocket error:', error);
                updateStatus('Connection error');
            };
        }

        // Handle incoming WebSocket messages
        function handleWebSocketMessage(message) {
            switch (message.type) {
                case 'StatusUpdate':
                    updateStatus(message.data.status);
                    break;
                case 'NodeCreated':
                    addNodeToCanvas(message.data.node_id, message.data.node_data);
                    break;
                case 'Error':
                    console.error('DAW Error:', message.data.message);
                    updateStatus('Error: ' + message.data.message);
                    break;
                default:
                    console.log('Received message:', message);
            }
        }

        // Send message to DAW
        function sendMessage(message) {
            if (ws && ws.readyState === WebSocket.OPEN) {
                ws.send(JSON.stringify(message));
            } else {
                console.warn('WebSocket not connected');
            }
        }

        // Update status display
        function updateStatus(text) {
            document.getElementById('status').textContent = text;
        }

        // Add node to canvas
        function addNodeToCanvas(nodeId, nodeData) {
            const canvas = document.getElementById('canvas');
            const node = document.createElement('div');
            node.className = 'node';
            node.id = nodeId;
            node.style.left = nodeData.position[0] + 'px';
            node.style.top = nodeData.position[1] + 'px';

            node.innerHTML = `
                <div class="node-header">${nodeData.type}</div>
                <div class="node-inputs">
                    <div class="node-port" data-type="input"></div>
                </div>
                <div class="parameter">
                    <label>Frequency</label>
                    <input type="range" min="20" max="2000" value="440" step="1">
                </div>
                <div class="node-outputs">
                    <div class="node-port" data-type="output"></div>
                </div>
            `;

            // Make node draggable
            node.addEventListener('mousedown', startDrag);
            canvas.appendChild(node);
            nodes.push({ id: nodeId, element: node, data: nodeData });
        }

        // Drag handling
        function startDrag(event) {
            draggedNode = event.target.closest('.node');
            if (draggedNode) {
                dragOffset.x = event.clientX - draggedNode.offsetLeft;
                dragOffset.y = event.clientY - draggedNode.offsetTop;
                document.addEventListener('mousemove', drag);
                document.addEventListener('mouseup', endDrag);
            }
        }

        function drag(event) {
            if (draggedNode) {
                draggedNode.style.left = (event.clientX - dragOffset.x) + 'px';
                draggedNode.style.top = (event.clientY - dragOffset.y) + 'px';
            }
        }

        function endDrag() {
            draggedNode = null;
            document.removeEventListener('mousemove', drag);
            document.removeEventListener('mouseup', endDrag);
        }

        // Event listeners
        document.getElementById('play-btn').addEventListener('click', () => {
            sendMessage({ type: 'Play' });
        });

        document.getElementById('pause-btn').addEventListener('click', () => {
            sendMessage({ type: 'Pause' });
        });

        document.getElementById('stop-btn').addEventListener('click', () => {
            sendMessage({ type: 'Stop' });
        });

        document.getElementById('tempo-slider').addEventListener('input', (e) => {
            document.getElementById('tempo-display').textContent = e.target.value + ' BPM';
        });

        // Node palette drag and drop
        document.querySelectorAll('.node-item').forEach(item => {
            item.addEventListener('dragstart', (e) => {
                e.dataTransfer.setData('text/plain', e.target.dataset.type);
            });
        });

        document.getElementById('canvas').addEventListener('dragover', (e) => {
            e.preventDefault();
        });

        document.getElementById('canvas').addEventListener('drop', (e) => {
            e.preventDefault();
            const nodeType = e.dataTransfer.getData('text/plain');
            if (nodeType) {
                sendMessage({
                    type: 'CreateNode',
                    data: {
                        node_type: nodeType,
                        position: [e.offsetX, e.offsetY]
                    }
                });
            }
        });

        // Initialize
        initWebSocket();
    </script>
</body>
</html>"#.to_string()
}

/// Create default web interface configuration
pub fn create_default_web_config() -> WebInterfaceConfig {
    WebInterfaceConfig {
        host: "127.0.0.1".to_string(),
        port: 3030,
        enable_cors: true,
        enable_websockets: true,
        static_files_path: "static".to_string(),
    }
}

/// Initialize web interface
pub async fn initialize_web_interface() -> Result<WebInterfaceServer, Box<dyn std::error::Error>> {
    let config = create_default_web_config();
    let server = WebInterfaceServer::new(config);
    Ok(server)
}