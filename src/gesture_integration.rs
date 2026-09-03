//! Gesture Integration Module
//!
//! This module provides integration with gesture control systems including:
//! - Leap Motion hand tracking
//! - MediaPipe pose estimation
//! - Custom gesture recognition

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

/// Gesture types supported by the system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GestureType {
    HandOpen,
    HandClosed,
    HandPoint,
    HandPinch,
    HandThumbsUp,
    HandPeace,
    HandRock,
    PoseStanding,
    PoseSitting,
    PoseWalking,
    PoseRunning,
    Custom(String),
}

/// Gesture data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureData {
    pub gesture_type: GestureType,
    pub confidence: f32,
    pub position: (f32, f32, f32), // x, y, z coordinates
    pub velocity: (f32, f32, f32), // velocity in each axis
    pub orientation: (f32, f32, f32, f32), // quaternion
    pub timestamp: f64,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Gesture mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureMapping {
    pub gesture_type: GestureType,
    pub target_type: String, // "audio", "visualization", "node", etc.
    pub target_parameter: String,
    pub min_value: f32,
    pub max_value: f32,
    pub smoothing: f32,
    pub enabled: bool,
}

/// Leap Motion integration
pub struct LeapMotionController {
    is_connected: bool,
    current_gestures: Vec<GestureData>,
    mappings: HashMap<String, GestureMapping>,
}

impl LeapMotionController {
    pub fn new() -> Self {
        Self {
            is_connected: false,
            current_gestures: Vec::new(),
            mappings: HashMap::new(),
        }
    }

    /// Initialize Leap Motion connection
    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // In a real implementation, this would connect to Leap Motion WebSocket API
        println!("🤖 Initializing Leap Motion controller...");
        self.is_connected = true;
        println!("✅ Leap Motion controller initialized");
        Ok(())
    }

    /// Update gesture data from Leap Motion
    pub fn update_gestures(&mut self, gestures: Vec<GestureData>) {
        self.current_gestures = gestures;
    }

    /// Get current gestures
    pub fn get_current_gestures(&self) -> &[GestureData] {
        &self.current_gestures
    }

    /// Add gesture mapping
    pub fn add_mapping(&mut self, id: String, mapping: GestureMapping) {
        self.mappings.insert(id, mapping);
    }

    /// Apply gesture mappings to targets
    pub fn apply_mappings(&self) -> HashMap<String, f32> {
        let mut results = HashMap::new();

        for gesture in &self.current_gestures {
            if let Some(mapping) = self.mappings.values().find(|m| m.gesture_type == gesture.gesture_type && m.enabled) {
                let value = self.map_gesture_to_value(gesture, mapping);
                results.insert(format!("{}.{}", mapping.target_type, mapping.target_parameter), value);
            }
        }

        results
    }

    /// Map gesture data to parameter value
    fn map_gesture_to_value(&self, gesture: &GestureData, mapping: &GestureMapping) -> f32 {
        // Use position Y coordinate as primary mapping value (height)
        let raw_value = gesture.position.1;
        let normalized_value = (raw_value + 200.0) / 400.0; // Assuming Leap Motion range
        let clamped_value = normalized_value.max(0.0).min(1.0);

        // Apply smoothing (simple exponential smoothing)
        // In a real implementation, you'd track previous values
        let smoothed_value = clamped_value;

        // Scale to target range
        mapping.min_value + (smoothed_value * (mapping.max_value - mapping.min_value))
    }
}

/// MediaPipe integration
pub struct MediaPipeController {
    is_initialized: bool,
    current_pose: Option<GestureData>,
    mappings: HashMap<String, GestureMapping>,
}

impl MediaPipeController {
    pub fn new() -> Self {
        Self {
            is_initialized: false,
            current_pose: None,
            mappings: HashMap::new(),
        }
    }

    /// Initialize MediaPipe
    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // In a real implementation, this would load MediaPipe models
        println!("🤖 Initializing MediaPipe controller...");
        self.is_initialized = true;
        println!("✅ MediaPipe controller initialized");
        Ok(())
    }

    /// Update pose data from MediaPipe
    pub fn update_pose(&mut self, pose: GestureData) {
        self.current_pose = Some(pose);
    }

    /// Get current pose
    pub fn get_current_pose(&self) -> Option<&GestureData> {
        self.current_pose.as_ref()
    }

    /// Add pose mapping
    pub fn add_mapping(&mut self, id: String, mapping: GestureMapping) {
        self.mappings.insert(id, mapping);
    }

    /// Apply pose mappings
    pub fn apply_mappings(&self) -> HashMap<String, f32> {
        let mut results = HashMap::new();

        if let Some(pose) = &self.current_pose {
            for mapping in self.mappings.values().filter(|m| m.enabled) {
                let value = self.map_pose_to_value(pose, mapping);
                results.insert(format!("{}.{}", mapping.target_type, mapping.target_parameter), value);
            }
        }

        results
    }

    /// Map pose data to parameter value
    fn map_pose_to_value(&self, pose: &GestureData, mapping: &GestureMapping) -> f32 {
        // Use different pose metrics based on mapping
        let raw_value = match mapping.gesture_type {
            GestureType::PoseStanding => pose.position.1, // Height for standing
            GestureType::PoseSitting => pose.position.1,  // Height for sitting
            _ => pose.confidence, // Use confidence for other poses
        };

        let normalized_value = raw_value.max(0.0).min(1.0);
        mapping.min_value + (normalized_value * (mapping.max_value - mapping.min_value))
    }
}

/// Main gesture integration controller
pub struct GestureIntegrationController {
    leap_motion: LeapMotionController,
    media_pipe: MediaPipeController,
    custom_gestures: HashMap<String, GestureMapping>,
    is_enabled: bool,
}

impl GestureIntegrationController {
    pub fn new() -> Self {
        Self {
            leap_motion: LeapMotionController::new(),
            media_pipe: MediaPipeController::new(),
            custom_gestures: HashMap::new(),
            is_enabled: false,
        }
    }

    /// Initialize all gesture controllers
    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🤖 Initializing Gesture Integration Controller...");

        self.leap_motion.initialize().await?;
        self.media_pipe.initialize().await?;

        // Initialize default mappings
        self.initialize_default_mappings();

        self.is_enabled = true;
        println!("✅ Gesture Integration Controller initialized");
        Ok(())
    }

    /// Initialize default gesture mappings
    fn initialize_default_mappings(&mut self) {
        // Leap Motion mappings
        self.leap_motion.add_mapping(
            "hand_volume".to_string(),
            GestureMapping {
                gesture_type: GestureType::HandOpen,
                target_type: "audio".to_string(),
                target_parameter: "master_volume".to_string(),
                min_value: 0.0,
                max_value: 1.0,
                smoothing: 0.8,
                enabled: true,
            }
        );

        // MediaPipe mappings
        self.media_pipe.add_mapping(
            "pose_filter".to_string(),
            GestureMapping {
                gesture_type: GestureType::PoseStanding,
                target_type: "audio".to_string(),
                target_parameter: "filter_cutoff".to_string(),
                min_value: 200.0,
                max_value: 8000.0,
                smoothing: 0.6,
                enabled: true,
            }
        );
    }

    /// Update gesture data
    pub fn update_gestures(&mut self, leap_gestures: Vec<GestureData>, media_pipe_pose: Option<GestureData>) {
        if !self.is_enabled {
            return;
        }

        self.leap_motion.update_gestures(leap_gestures);

        if let Some(pose) = media_pipe_pose {
            self.media_pipe.update_pose(pose);
        }
    }

    /// Get all current gesture values
    pub fn get_gesture_values(&self) -> HashMap<String, f32> {
        let mut values = HashMap::new();

        // Get Leap Motion values
        let leap_values = self.leap_motion.apply_mappings();
        values.extend(leap_values);

        // Get MediaPipe values
        let media_pipe_values = self.media_pipe.apply_mappings();
        values.extend(media_pipe_values);

        values
    }

    /// Add custom gesture mapping
    pub fn add_custom_mapping(&mut self, id: String, mapping: GestureMapping) {
        self.custom_gestures.insert(id, mapping);
    }

    /// Enable/disable gesture control
    pub fn set_enabled(&mut self, enabled: bool) {
        self.is_enabled = enabled;
        println!("🤖 Gesture control {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Check if gesture control is enabled
    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    /// Get status information
    pub fn get_status(&self) -> GestureStatus {
        GestureStatus {
            leap_motion_connected: self.leap_motion.is_connected,
            media_pipe_initialized: self.media_pipe.is_initialized,
            enabled: self.is_enabled,
            active_mappings: self.leap_motion.mappings.len() + self.media_pipe.mappings.len() + self.custom_gestures.len(),
        }
    }
}

/// Gesture system status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureStatus {
    pub leap_motion_connected: bool,
    pub media_pipe_initialized: bool,
    pub enabled: bool,
    pub active_mappings: usize,
}

/// Gesture recognition utilities
pub struct GestureRecognitionUtils;

impl GestureRecognitionUtils {
    /// Recognize gesture from hand landmarks
    pub fn recognize_hand_gesture(landmarks: &[(f32, f32, f32)]) -> Option<GestureType> {
        if landmarks.len() < 21 {
            return None;
        }

        // Simple gesture recognition based on finger positions
        // This is a simplified implementation - real recognition would be more sophisticated

        let thumb_tip = landmarks[4];
        let index_tip = landmarks[8];
        let middle_tip = landmarks[12];
        let ring_tip = landmarks[16];
        let pinky_tip = landmarks[20];

        // Check if fingers are extended
        let fingers_extended = [
            index_tip.1 < landmarks[6].1,   // Index finger
            middle_tip.1 < landmarks[10].1, // Middle finger
            ring_tip.1 < landmarks[14].1,   // Ring finger
            pinky_tip.1 < landmarks[18].1,  // Pinky finger
        ];

        let extended_count = fingers_extended.iter().filter(|&&x| x).count();

        match extended_count {
            0 => Some(GestureType::HandClosed),
            1 => Some(GestureType::HandPoint),
            2 => Some(GestureType::HandPeace),
            4 => Some(GestureType::HandOpen),
            _ => Some(GestureType::HandThumbsUp), // Default fallback
        }
    }

    /// Recognize pose from keypoints
    pub fn recognize_pose(keypoints: &[(f32, f32, f32)]) -> Option<GestureType> {
        if keypoints.len() < 17 { // COCO format has 17 keypoints
            return None;
        }

        // Simple pose recognition based on keypoint positions
        let nose = keypoints[0];
        let left_shoulder = keypoints[5];
        let right_shoulder = keypoints[6];
        let left_hip = keypoints[11];
        let right_hip = keypoints[12];

        // Calculate approximate pose metrics
        let shoulder_width = (right_shoulder.0 - left_shoulder.0).abs();
        let hip_width = (right_hip.0 - left_hip.0).abs();
        let body_height = (nose.1 - (left_hip.1 + right_hip.1) / 2.0).abs();

        // Simple classification
        if body_height > shoulder_width * 2.0 {
            Some(GestureType::PoseStanding)
        } else if body_height < shoulder_width * 1.5 {
            Some(GestureType::PoseSitting)
        } else {
            Some(GestureType::PoseWalking)
        }
    }
}

/// Integration with the main DAW system
pub fn create_gesture_integration() -> Arc<Mutex<GestureIntegrationController>> {
    Arc::new(Mutex::new(GestureIntegrationController::new()))
}