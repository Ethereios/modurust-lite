/// Time-stamped Event Queue - Sample-accurate parameter automation
/// 
/// This module implements a lock-free, real-time safe event queue for
/// sample-accurate parameter automation in the audio engine.
///
/// Enhanced to support node-specific parameter changes with precise timing
/// for professional-level DAW capabilities.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use crossbeam_utils::atomic::AtomicCell;


/// Maximum number of events in the queue
const MAX_EVENTS: usize = 8192; // Increased for more complex automation

/// Event types that can be scheduled
#[derive(Debug, Clone, Copy)]
pub enum EventType {
    /// Parameter change with node identification
    ParamChange {
        /// Node ID
        node_id: u32,
        /// Parameter ID
        param_id: u32,
        /// New parameter value
        value: f32,
        /// Optional curve type for interpolation
        curve_type: CurveType,
    },
    /// MIDI event
    MidiEvent {
        /// MIDI status byte
        status: u8,
        /// MIDI data byte 1
        data1: u8,
        /// MIDI data byte 2
        data2: u8,
        /// MIDI channel
        channel: u8,
        /// Target node ID
        target_node_id: u32,
    },
    /// Transport event
    TransportEvent {
        /// Transport command
        command: u8,
        /// Transport value
        value: f32,
        /// Additional context data
        context: u32,
    },
    /// Scene change event
    SceneChange {
        /// Scene ID
        scene_id: u32,
        /// Crossfade time in milliseconds
        crossfade_ms: u32,
    },
    /// Node state change
    NodeStateChange {
        /// Node ID
        node_id: u32,
        /// State change type
        change_type: NodeStateChangeType,
    },
}

/// Curve types for parameter automation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurveType {
    /// Linear interpolation
    Linear,
    /// Exponential curve
    Exponential,
    /// Logarithmic curve
    Logarithmic,
    /// S-curve (sigmoid)
    SCurve,
    /// Step (no interpolation)
    Step,
}

/// Node state change types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeStateChangeType {
    /// Activate node
    Activate,
    /// Deactivate node
    Deactivate,
    /// Replace node state with snapshot
    ReplaceState(u32), // snapshot_id
}

/// Time-stamped event
#[derive(Debug, Clone, Copy)]
pub struct TimedEvent {
    /// Sample offset within the current buffer
    pub sample_offset: u32,
    /// Absolute sample position in the timeline
    pub timeline_sample: u64,
    /// Event type and data
    pub event_type: EventType,
    /// Priority level (higher values are processed first when events occur at the same sample)
    pub priority: u8,
    /// Event ID for tracking and cancellation
    pub event_id: u32,
}

/// Lock-free event queue for real-time audio processing
pub struct EventQueue {
    /// Events storage
    events: Box<[AtomicCell<Option<TimedEvent>>]>,
    /// Write index
    write_idx: AtomicUsize,
    /// Read index
    read_idx: AtomicUsize,
    /// Size mask for power-of-two ring buffer
    size_mask: usize,
    /// Next event ID
    next_event_id: AtomicUsize,
}

impl EventQueue {
    /// Create a new event queue
    pub fn new() -> Self {
        // Ensure size is power of two for efficient masking
        let size = MAX_EVENTS.next_power_of_two();
        let size_mask = size - 1;
        
        // Create array of atomic cells
        let mut events = Vec::with_capacity(size);
        for _ in 0..size {
            events.push(AtomicCell::new(None));
        }
        
        Self {
            events: events.into_boxed_slice(),
            write_idx: AtomicUsize::new(0),
            read_idx: AtomicUsize::new(0),
            size_mask,
            next_event_id: AtomicUsize::new(1), // Start from 1, 0 is reserved for invalid ID
        }
    }
    
    /// Generate a new event ID
    fn generate_event_id(&self) -> u32 {
        let id = self.next_event_id.fetch_add(1, Ordering::Relaxed);
        // Wrap around if we reach u32::MAX
        (id % u32::MAX as usize) as u32
    }
    
    /// Push an event to the queue (from UI thread)
    /// Returns true if successful, false if queue is full
    pub fn push(&self, mut event: TimedEvent) -> bool {
        let write_idx = self.write_idx.load(Ordering::Relaxed);
        let read_idx = self.read_idx.load(Ordering::Acquire);
        
        // Check if queue is full
        if write_idx - read_idx >= self.events.len() {
            return false;
        }
        
        // Assign event ID if not already set
        if event.event_id == 0 {
            event.event_id = self.generate_event_id();
        }
        
        // Write event to the queue
        let idx = write_idx & self.size_mask;
        self.events[idx].store(Some(event));
        
        // Update write index with release ordering to ensure visibility
        self.write_idx.store(write_idx + 1, Ordering::Release);
        
        true
    }
    
    /// Cancel events by node ID and parameter ID
    /// Returns the number of events canceled
    pub fn cancel_events_by_node_param(&self, node_id: u32, param_id: u32) -> usize {
        let mut canceled = 0;
        let write_idx = self.write_idx.load(Ordering::Acquire);
        let read_idx = self.read_idx.load(Ordering::Acquire);
        
        for i in read_idx..write_idx {
            let idx = i & self.size_mask;
            if let Some(event) = self.events[idx].load() {
                if let EventType::ParamChange { node_id: n_id, param_id: p_id, .. } = event.event_type {
                    if n_id == node_id && p_id == param_id {
                        // Replace with None to cancel the event
                        self.events[idx].store(None);
                        canceled += 1;
                    }
                }
            }
        }
        
        canceled
    }
    
    /// Pop an event from the queue (from audio thread)
    /// Returns None if queue is empty
    pub fn pop(&self) -> Option<TimedEvent> {
        let read_idx = self.read_idx.load(Ordering::Relaxed);
        let write_idx = self.write_idx.load(Ordering::Acquire);
        
        // Check if queue is empty
        if read_idx >= write_idx {
            return None;
        }
        
        // Read event from the queue
        let idx = read_idx & self.size_mask;
        let event = self.events[idx].swap(None);
        
        // Update read index with release ordering
        self.read_idx.store(read_idx + 1, Ordering::Release);
        
        event
    }
    
    /// Get all events for the current audio buffer
    /// Returns events sorted by sample offset
    pub fn get_buffer_events(&self, buffer_size: usize, current_sample: u64) -> Vec<TimedEvent> {
        let mut events = Vec::new();
        let buffer_end_sample = current_sample + buffer_size as u64;
        
        // Collect all events that fall within this buffer
        while let Some(event) = self.pop() {
            if event.timeline_sample >= current_sample && event.timeline_sample < buffer_end_sample {
                events.push(event);
            } else if event.timeline_sample >= buffer_end_sample {
                // This event is for a future buffer, put it back
                self.push(event);
                break;
            }
        }
        
        // Sort events by sample offset
        events.sort_by_key(|e| e.sample_offset);
        
        events
    }
    
    /// Clear all events from the queue
    pub fn clear(&self) {
        let write_idx = self.write_idx.load(Ordering::Relaxed);
        let read_idx = self.read_idx.load(Ordering::Relaxed);
        
        // Clear all cells between read and write indices
        for i in read_idx..write_idx {
            let idx = i & self.size_mask;
            self.events[idx].store(None);
        }
        
        // Reset indices
        self.read_idx.store(write_idx, Ordering::Release);
    }
    
    /// Get the number of events in the queue
    pub fn len(&self) -> usize {
        let write_idx = self.write_idx.load(Ordering::Acquire);
        let read_idx = self.read_idx.load(Ordering::Acquire);
        write_idx - read_idx
    }
    
    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Helper for scheduling parameter automation events
pub struct ParameterAutomation {
    /// Event queue
    event_queue: Arc<EventQueue>,
    /// Current timeline sample position
    current_sample: u64,
}

impl ParameterAutomation {
    /// Create a new parameter automation helper
    pub fn new(event_queue: Arc<EventQueue>) -> Self {
        Self {
            event_queue,
            current_sample: 0,
        }
    }
    
    /// Update current timeline position
    pub fn update_position(&mut self, current_sample: u64) {
        self.current_sample = current_sample;
    }
    
    /// Schedule a parameter change
    pub fn schedule_param_change(&self, node_id: u32, param_id: u32, value: f32, sample_offset: u32) -> bool {
        let timeline_sample = self.current_sample + sample_offset as u64;
        
        let event = TimedEvent {
            sample_offset,
            timeline_sample,
            event_type: EventType::ParamChange { node_id, param_id, value, curve_type: CurveType::Linear },
            priority: 0,
            event_id: 0,
        };
        
        self.event_queue.push(event)
    }
    
    /// Schedule a MIDI event
    pub fn schedule_midi_event(&self, status: u8, data1: u8, data2: u8, channel: u8, target_node_id: u32, sample_offset: u32) -> bool {
        let timeline_sample = self.current_sample + sample_offset as u64;
        
        let event = TimedEvent {
            sample_offset,
            timeline_sample,
            event_type: EventType::MidiEvent { status, data1, data2, channel, target_node_id },
            priority: 0,
            event_id: 0,
        };
        
        self.event_queue.push(event)
    }
    
    /// Schedule a transport event
    pub fn schedule_transport_event(&self, command: u8, value: f32, context: u32, sample_offset: u32) -> bool {
        let timeline_sample = self.current_sample + sample_offset as u64;
        
        let event = TimedEvent {
            sample_offset,
            timeline_sample,
            event_type: EventType::TransportEvent { command, value, context },
            priority: 0,
            event_id: 0,
        };
        
        self.event_queue.push(event)
    }
}

/// Safe wrapper for processing events in the audio thread
pub struct AudioThreadEventProcessor {
    /// Event queue
    event_queue: Arc<EventQueue>,
    /// Current timeline sample position
    current_sample: u64,
}

impl AudioThreadEventProcessor {
    /// Create a new audio thread event processor
    pub fn new(event_queue: Arc<EventQueue>) -> Self {
        Self {
            event_queue,
            current_sample: 0,
        }
    }
    
    /// Process events for the current audio buffer
    pub fn process_buffer<F>(&mut self, buffer_size: usize, mut process_event: F)
    where
        F: FnMut(&EventType, u32),
    {
        // Get events for this buffer
        let events = self.event_queue.get_buffer_events(buffer_size, self.current_sample);
        
        // Process each event
        for event in events {
            process_event(&event.event_type, event.sample_offset);
        }
        
        // Update current sample position
        self.current_sample += buffer_size as u64;
    }
    
    /// Get current timeline sample position
    pub fn get_current_sample(&self) -> u64 {
        self.current_sample
    }
    
    /// Set current timeline sample position
    pub fn set_current_sample(&mut self, sample: u64) {
        self.current_sample = sample;
    }
}