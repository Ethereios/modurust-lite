use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Comprehensive piano roll editor for MIDI note editing and sequencing
/// Provides professional-grade MIDI editing capabilities

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PianoRollEditor {
    pub id: Uuid,
    pub name: String,
    pub midi_clip: MidiClip,
    pub view_settings: ViewSettings,
    pub edit_settings: EditSettings,
    pub selection: Selection,
    pub clipboard: Vec<MidiNote>,
    pub undo_stack: Vec<EditorAction>,
    pub redo_stack: Vec<EditorAction>,
    pub quantize_settings: QuantizeSettings,
    pub velocity_editor: VelocityEditor,
    pub automation_lanes: Vec<AutomationLane>,
    pub scale_settings: ScaleSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiClip {
    pub id: Uuid,
    pub name: String,
    pub notes: Vec<MidiNote>,
    pub length_beats: f64,
    pub time_signature: TimeSignature,
    pub key_signature: KeySignature,
    pub tempo: f64,
    pub swing: f32,
    pub groove_template: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiNote {
    pub id: Uuid,
    pub pitch: u8,           // MIDI note number (0-127)
    pub velocity: u8,        // Velocity (0-127)
    pub start_time: f64,     // Start time in beats
    pub duration: f64,       // Duration in beats
    pub channel: u8,         // MIDI channel (0-15)
    pub selected: bool,
    pub muted: bool,
    pub color: Option<String>,
    pub articulation: Option<Articulation>,
    pub micro_timing: f64,   // Fine timing adjustment in ticks
    pub probability: f32,    // Note probability (0.0-1.0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Articulation {
    pub name: String,
    pub velocity_offset: i8,
    pub timing_offset: f64,
    pub duration_multiplier: f32,
    pub cc_changes: HashMap<u8, u8>, // CC number -> value
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSignature {
    pub numerator: u8,
    pub denominator: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeySignature {
    pub key: String,        // "C", "G", "D", etc.
    pub mode: String,       // "Major", "Minor", etc.
    pub accidentals: i8,    // -7 to +7 (flats to sharps)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewSettings {
    pub zoom_horizontal: f32,    // Horizontal zoom level
    pub zoom_vertical: f32,      // Vertical zoom level
    pub scroll_position: (f64, f64), // (time, pitch)
    pub visible_range: PitchRange,
    pub snap_to_grid: bool,
    pub grid_resolution: GridResolution,
    pub show_velocity: bool,
    pub show_note_names: bool,
    pub show_octave_lines: bool,
    pub show_scale_highlighting: bool,
    pub color_mode: ColorMode,
    pub note_display_mode: NoteDisplayMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchRange {
    pub min_pitch: u8,
    pub max_pitch: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GridResolution {
    Whole,          // 1/1
    Half,           // 1/2
    Quarter,        // 1/4
    Eighth,         // 1/8
    Sixteenth,      // 1/16
    ThirtySecond,   // 1/32
    SixtyFourth,    // 1/64
    Triplet(Box<GridResolution>), // Triplet of any resolution
    Dotted(Box<GridResolution>),  // Dotted of any resolution
    Custom(f64),    // Custom resolution in beats
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColorMode {
    Velocity,       // Color by velocity
    Pitch,          // Color by pitch
    Channel,        // Color by MIDI channel
    Timing,         // Color by timing (early/late)
    Articulation,   // Color by articulation
    Uniform(String), // Single color
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoteDisplayMode {
    Blocks,         // Traditional rectangular blocks
    Diamonds,       // Diamond shapes
    Lines,          // Line representation
    Dots,           // Dot representation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditSettings {
    pub tool: EditTool,
    pub insert_velocity: u8,
    pub insert_duration: f64,
    pub auto_scroll: bool,
    pub loop_playback: bool,
    pub metronome_enabled: bool,
    pub count_in_bars: u8,
    pub record_mode: RecordMode,
    pub input_quantize: bool,
    pub step_record: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditTool {
    Select,         // Selection tool
    Draw,           // Draw/pencil tool
    Erase,          // Eraser tool
    Split,          // Split/cut tool
    Glue,           // Glue/join tool
    Velocity,       // Velocity editing tool
    Timing,         // Timing adjustment tool
    Pitch,          // Pitch bend tool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecordMode {
    Replace,        // Replace existing notes
    Overdub,        // Add to existing notes
    Merge,          // Merge with existing notes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub notes: Vec<Uuid>,
    pub time_range: Option<(f64, f64)>,
    pub pitch_range: Option<(u8, u8)>,
    pub selection_mode: SelectionMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SelectionMode {
    Individual,     // Select individual notes
    Rectangle,      // Rectangle selection
    Lasso,          // Lasso selection
    All,            // Select all
    Similar,        // Select similar notes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizeSettings {
    pub enabled: bool,
    pub strength: f32,      // 0.0 to 1.0
    pub grid: GridResolution,
    pub swing: f32,         // -100% to +100%
    pub humanize: HumanizeSettings,
    pub quantize_start: bool,
    pub quantize_end: bool,
    pub quantize_velocity: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanizeSettings {
    pub timing_variance: f64,    // Random timing variance in ticks
    pub velocity_variance: u8,   // Random velocity variance
    pub duration_variance: f64,  // Random duration variance
    pub pitch_variance: u8,      // Random pitch variance (for humanization)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelocityEditor {
    pub visible: bool,
    pub height: f32,
    pub edit_mode: VelocityEditMode,
    pub curve_type: CurveType,
    pub velocity_range: (u8, u8),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VelocityEditMode {
    Individual,     // Edit individual note velocities
    Curve,          // Draw velocity curves
    Ramp,           // Create velocity ramps
    Random,         // Randomize velocities
    Scale,          // Scale velocities
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CurveType {
    Linear,
    Exponential,
    Logarithmic,
    Sine,
    Cosine,
    Custom(Vec<(f64, f32)>), // Custom curve points
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationLane {
    pub id: Uuid,
    pub name: String,
    pub parameter: AutomationParameter,
    pub points: Vec<AutomationPoint>,
    pub visible: bool,
    pub height: f32,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutomationParameter {
    CC(u8),                 // MIDI CC
    Pitchbend,
    Aftertouch,
    ChannelPressure,
    ProgramChange,
    Custom(String),         // Custom parameter
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationPoint {
    pub time: f64,          // Time in beats
    pub value: f32,         // Normalized value (0.0-1.0)
    pub curve: CurveType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaleSettings {
    pub enabled: bool,
    pub scale: Scale,
    pub root_note: u8,
    pub highlight_mode: ScaleHighlightMode,
    pub snap_to_scale: bool,
    pub chord_detection: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scale {
    pub name: String,
    pub intervals: Vec<u8>,  // Semitone intervals from root
    pub mode_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScaleHighlightMode {
    None,
    InScale,        // Highlight notes in scale
    OutOfScale,     // Highlight notes out of scale
    ChordTones,     // Highlight chord tones
    Tensions,       // Highlight tensions/extensions
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditorAction {
    AddNote(MidiNote),
    RemoveNote(Uuid),
    ModifyNote { id: Uuid, old: MidiNote, new: MidiNote },
    MoveNotes { notes: Vec<Uuid>, delta_time: f64, delta_pitch: i8 },
    QuantizeNotes { notes: Vec<Uuid>, settings: QuantizeSettings },
    ChangeVelocity { notes: Vec<Uuid>, old_velocities: Vec<u8>, new_velocities: Vec<u8> },
    ChangeDuration { notes: Vec<Uuid>, old_durations: Vec<f64>, new_durations: Vec<f64> },
    Paste { notes: Vec<MidiNote>, position: f64 },
    Delete { notes: Vec<MidiNote> },
}

impl PianoRollEditor {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            midi_clip: MidiClip::new("New Clip".to_string()),
            view_settings: ViewSettings::default(),
            edit_settings: EditSettings::default(),
            selection: Selection::default(),
            clipboard: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            quantize_settings: QuantizeSettings::default(),
            velocity_editor: VelocityEditor::default(),
            automation_lanes: Vec::new(),
            scale_settings: ScaleSettings::default(),
        }
    }

    pub fn add_note(&mut self, pitch: u8, start_time: f64, duration: f64, velocity: u8) -> Uuid {
        let note = MidiNote {
            id: Uuid::new_v4(),
            pitch,
            velocity,
            start_time,
            duration,
            channel: 0,
            selected: false,
            muted: false,
            color: None,
            articulation: None,
            micro_timing: 0.0,
            probability: 1.0,
        };

        let note_id = note.id;
        self.midi_clip.notes.push(note.clone());
        self.add_to_undo_stack(EditorAction::AddNote(note));
        note_id
    }

    pub fn remove_note(&mut self, note_id: Uuid) -> Result<(), String> {
        if let Some(index) = self.midi_clip.notes.iter().position(|n| n.id == note_id) {
            let _note = self.midi_clip.notes.remove(index);
            self.add_to_undo_stack(EditorAction::RemoveNote(note_id));
            Ok(())
        } else {
            Err("Note not found".to_string())
        }
    }

    pub fn select_notes_in_range(&mut self, time_start: f64, time_end: f64, pitch_start: u8, pitch_end: u8) {
        self.selection.notes.clear();
        
        for note in &mut self.midi_clip.notes {
            let in_time_range = note.start_time >= time_start && note.start_time <= time_end;
            let in_pitch_range = note.pitch >= pitch_start && note.pitch <= pitch_end;
            
            if in_time_range && in_pitch_range {
                note.selected = true;
                self.selection.notes.push(note.id);
            } else {
                note.selected = false;
            }
        }
    }

    pub fn quantize_selected_notes(&mut self) {
        let selected_notes: Vec<Uuid> = self.selection.notes.clone();
        // Precompute grid size and swing to avoid borrowing self during mutation
        let grid_size = self.get_grid_size();
        let swing = self.quantize_settings.swing as f64;

        for note_id in &selected_notes {
            if let Some(note) = self.midi_clip.notes.iter_mut().find(|n| n.id == *note_id) {
                let quantized = (note.start_time / grid_size).round() * grid_size;

                if swing != 0.0 {
                    let beat_position = note.start_time % (grid_size * 2.0);
                    if beat_position >= grid_size {
                        let swing_offset = grid_size * (swing / 100.0) * 0.1;
                        note.start_time = quantized + swing_offset;
                        continue;
                    }
                }

                note.start_time = quantized;
            }
        }

        self.add_to_undo_stack(EditorAction::QuantizeNotes {
            notes: selected_notes,
            settings: self.quantize_settings.clone(),
        });
    }

    #[allow(dead_code)]
    fn quantize_time(&self, time: f64) -> f64 {
        let grid_size = self.get_grid_size();
        let quantized = (time / grid_size).round() * grid_size;
        
        // Apply swing
        if self.quantize_settings.swing != 0.0 {
            let beat_position = time % (grid_size * 2.0);
            if beat_position >= grid_size {
                let swing_offset = grid_size * ((self.quantize_settings.swing as f64) / 100.0) * 0.1;
                return quantized + swing_offset;
            }
        }
        
        quantized
    }

    fn get_grid_size(&self) -> f64 {
        match &self.quantize_settings.grid {
            GridResolution::Whole => 4.0,
            GridResolution::Half => 2.0,
            GridResolution::Quarter => 1.0,
            GridResolution::Eighth => 0.5,
            GridResolution::Sixteenth => 0.25,
            GridResolution::ThirtySecond => 0.125,
            GridResolution::SixtyFourth => 0.0625,
            GridResolution::Triplet(base) => {
                let base_size = match base.as_ref() {
                    GridResolution::Quarter => 1.0,
                    GridResolution::Eighth => 0.5,
                    _ => 1.0,
                };
                base_size * 2.0 / 3.0
            },
            GridResolution::Dotted(base) => {
                let base_size = match base.as_ref() {
                    GridResolution::Quarter => 1.0,
                    GridResolution::Eighth => 0.5,
                    _ => 1.0,
                };
                base_size * 1.5
            },
            GridResolution::Custom(size) => *size,
        }
    }

    pub fn transpose_selected_notes(&mut self, semitones: i8) {
        let selected_notes: Vec<Uuid> = self.selection.notes.clone();
        
        for note_id in &selected_notes {
            if let Some(note) = self.midi_clip.notes.iter_mut().find(|n| n.id == *note_id) {
                let new_pitch = (note.pitch as i16 + semitones as i16).clamp(0, 127) as u8;
                note.pitch = new_pitch;
            }
        }
    }

    pub fn scale_velocities(&mut self, factor: f32) {
        let selected_notes: Vec<Uuid> = self.selection.notes.clone();
        let mut old_velocities = Vec::new();
        let mut new_velocities = Vec::new();
        
        for note_id in &selected_notes {
            if let Some(note) = self.midi_clip.notes.iter_mut().find(|n| n.id == *note_id) {
                old_velocities.push(note.velocity);
                let new_velocity = ((note.velocity as f32 * factor).clamp(1.0, 127.0)) as u8;
                note.velocity = new_velocity;
                new_velocities.push(new_velocity);
            }
        }
        
        self.add_to_undo_stack(EditorAction::ChangeVelocity {
            notes: selected_notes,
            old_velocities,
            new_velocities,
        });
    }

    pub fn humanize_selected_notes(&mut self) {
        let humanize = &self.quantize_settings.humanize;
        let selected_notes: Vec<Uuid> = self.selection.notes.clone();
        
        for note_id in &selected_notes {
            if let Some(note) = self.midi_clip.notes.iter_mut().find(|n| n.id == *note_id) {
                // Randomize timing
                let timing_offset = (rand::random::<f64>() - 0.5) * humanize.timing_variance;
                note.micro_timing = timing_offset;
                
                // Randomize velocity
                let velocity_jitter = ((rand::random::<f32>() - 0.5) * 2.0) * humanize.velocity_variance as f32;
                let velocity_offset = velocity_jitter.round() as i16;
                note.velocity = ((note.velocity as i16 + velocity_offset).clamp(1, 127)) as u8;
                
                // Randomize duration
                let duration_offset = (rand::random::<f64>() - 0.5) * humanize.duration_variance;
                note.duration = (note.duration + duration_offset).max(0.01);
            }
        }
    }

    pub fn create_velocity_ramp(&mut self, start_velocity: u8, end_velocity: u8) {
        let selected_notes: Vec<Uuid> = self.selection.notes.clone();
        
        if selected_notes.len() < 2 {
            return;
        }
        
        // Sort notes by start time
        let mut note_times: Vec<(Uuid, f64)> = selected_notes.iter()
            .filter_map(|id| {
                self.midi_clip.notes.iter()
                    .find(|n| n.id == *id)
                    .map(|n| (*id, n.start_time))
            })
            .collect();
        
        note_times.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        
        let velocity_range = end_velocity as f64 - start_velocity as f64;
        let time_range = note_times.last().unwrap().1 - note_times.first().unwrap().1;
        
        for (i, (note_id, time)) in note_times.iter().enumerate() {
            if let Some(note) = self.midi_clip.notes.iter_mut().find(|n| n.id == *note_id) {
                let progress: f64 = if time_range > 0.0 {
                    (time - note_times.first().unwrap().1) / time_range
                } else {
                    i as f64 / (note_times.len() - 1) as f64
                };

                let new_velocity = (start_velocity as f64 + velocity_range * progress).clamp(1.0, 127.0).round() as u8;
                note.velocity = new_velocity;
            }
        }
    }

    pub fn add_automation_lane(&mut self, parameter: AutomationParameter, name: String) -> Uuid {
        let lane = AutomationLane {
            id: Uuid::new_v4(),
            name,
            parameter,
            points: Vec::new(),
            visible: true,
            height: 100.0,
            color: "#4FC3F7".to_string(),
        };
        
        let lane_id = lane.id;
        self.automation_lanes.push(lane);
        lane_id
    }

    pub fn add_automation_point(&mut self, lane_id: Uuid, time: f64, value: f32) -> Result<(), String> {
        if let Some(lane) = self.automation_lanes.iter_mut().find(|l| l.id == lane_id) {
            let point = AutomationPoint {
                time,
                value: value.clamp(0.0, 1.0),
                curve: CurveType::Linear,
            };
            
            // Insert in chronological order
            let insert_pos = lane.points.iter().position(|p| p.time > time).unwrap_or(lane.points.len());
            lane.points.insert(insert_pos, point);
            Ok(())
        } else {
            Err("Automation lane not found".to_string())
        }
    }

    pub fn copy_selected_notes(&mut self) {
        self.clipboard.clear();
        
        for note_id in &self.selection.notes {
            if let Some(note) = self.midi_clip.notes.iter().find(|n| n.id == *note_id) {
                self.clipboard.push(note.clone());
            }
        }
    }

    pub fn paste_notes(&mut self, position: f64) {
        if self.clipboard.is_empty() {
            return;
        }
        
        let mut new_notes = Vec::new();
        let min_time = self.clipboard.iter().map(|n| n.start_time).fold(f64::INFINITY, f64::min);
        let time_offset = position - min_time;
        
        for note in &self.clipboard {
            let mut new_note = note.clone();
            new_note.id = Uuid::new_v4();
            new_note.start_time += time_offset;
            new_note.selected = true;
            new_notes.push(new_note.clone());
            self.midi_clip.notes.push(new_note);
        }
        
        self.add_to_undo_stack(EditorAction::Paste { notes: new_notes, position });
    }

    pub fn undo(&mut self) -> Result<(), String> {
        if let Some(action) = self.undo_stack.pop() {
            self.apply_reverse_action(&action)?;
            self.redo_stack.push(action);
            Ok(())
        } else {
            Err("Nothing to undo".to_string())
        }
    }

    pub fn redo(&mut self) -> Result<(), String> {
        if let Some(action) = self.redo_stack.pop() {
            self.apply_action(&action)?;
            self.undo_stack.push(action);
            Ok(())
        } else {
            Err("Nothing to redo".to_string())
        }
    }

    fn add_to_undo_stack(&mut self, action: EditorAction) {
        self.undo_stack.push(action);
        self.redo_stack.clear(); // Clear redo stack when new action is performed
        
        // Limit undo stack size
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
    }

    fn apply_action(&mut self, action: &EditorAction) -> Result<(), String> {
        match action {
            EditorAction::AddNote(note) => {
                self.midi_clip.notes.push(note.clone());
            },
            EditorAction::RemoveNote(id) => {
                self.midi_clip.notes.retain(|n| n.id != *id);
            },
            // Implement other actions...
            _ => {},
        }
        Ok(())
    }

    fn apply_reverse_action(&mut self, action: &EditorAction) -> Result<(), String> {
        match action {
            EditorAction::AddNote(note) => {
                self.midi_clip.notes.retain(|n| n.id != note.id);
            },
            EditorAction::RemoveNote(_) => {
                // Would need to store the removed note to restore it
            },
            // Implement other reverse actions...
            _ => {},
        }
        Ok(())
    }

    pub fn get_notes_in_time_range(&self, start: f64, end: f64) -> Vec<&MidiNote> {
        self.midi_clip.notes.iter()
            .filter(|note| note.start_time < end && note.start_time + note.duration > start)
            .collect()
    }

    pub fn get_chord_at_time(&self, time: f64) -> Vec<u8> {
        self.midi_clip.notes.iter()
            .filter(|note| note.start_time <= time && note.start_time + note.duration > time)
            .map(|note| note.pitch)
            .collect()
    }
}

impl MidiClip {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            notes: Vec::new(),
            length_beats: 16.0,
            time_signature: TimeSignature { numerator: 4, denominator: 4 },
            key_signature: KeySignature {
                key: "C".to_string(),
                mode: "Major".to_string(),
                accidentals: 0,
            },
            tempo: 120.0,
            swing: 0.0,
            groove_template: None,
        }
    }
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            zoom_horizontal: 1.0,
            zoom_vertical: 1.0,
            scroll_position: (0.0, 60.0), // Start at middle C
            visible_range: PitchRange { min_pitch: 0, max_pitch: 127 },
            snap_to_grid: true,
            grid_resolution: GridResolution::Sixteenth,
            show_velocity: true,
            show_note_names: true,
            show_octave_lines: true,
            show_scale_highlighting: false,
            color_mode: ColorMode::Velocity,
            note_display_mode: NoteDisplayMode::Blocks,
        }
    }
}

impl Default for EditSettings {
    fn default() -> Self {
        Self {
            tool: EditTool::Select,
            insert_velocity: 100,
            insert_duration: 0.25, // Sixteenth note
            auto_scroll: true,
            loop_playback: false,
            metronome_enabled: true,
            count_in_bars: 1,
            record_mode: RecordMode::Replace,
            input_quantize: false,
            step_record: false,
        }
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            notes: Vec::new(),
            time_range: None,
            pitch_range: None,
            selection_mode: SelectionMode::Individual,
        }
    }
}

impl Default for QuantizeSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            strength: 1.0,
            grid: GridResolution::Sixteenth,
            swing: 0.0,
            humanize: HumanizeSettings::default(),
            quantize_start: true,
            quantize_end: false,
            quantize_velocity: false,
        }
    }
}

impl Default for HumanizeSettings {
    fn default() -> Self {
        Self {
            timing_variance: 10.0,  // 10 ticks
            velocity_variance: 5,   // ±5 velocity
            duration_variance: 0.02, // ±0.02 beats
            pitch_variance: 0,      // No pitch variance by default
        }
    }
}

impl Default for VelocityEditor {
    fn default() -> Self {
        Self {
            visible: false,
            height: 100.0,
            edit_mode: VelocityEditMode::Individual,
            curve_type: CurveType::Linear,
            velocity_range: (1, 127),
        }
    }
}

impl Default for ScaleSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            scale: Scale::major(),
            root_note: 60, // Middle C
            highlight_mode: ScaleHighlightMode::InScale,
            snap_to_scale: false,
            chord_detection: false,
        }
    }
}

impl Scale {
    pub fn major() -> Self {
        Self {
            name: "Major".to_string(),
            intervals: vec![0, 2, 4, 5, 7, 9, 11],
            mode_names: vec![
                "Ionian".to_string(),
                "Dorian".to_string(),
                "Phrygian".to_string(),
                "Lydian".to_string(),
                "Mixolydian".to_string(),
                "Aeolian".to_string(),
                "Locrian".to_string(),
            ],
        }
    }

    pub fn minor() -> Self {
        Self {
            name: "Natural Minor".to_string(),
            intervals: vec![0, 2, 3, 5, 7, 8, 10],
            mode_names: vec![
                "Aeolian".to_string(),
                "Locrian".to_string(),
                "Ionian".to_string(),
                "Dorian".to_string(),
                "Phrygian".to_string(),
                "Lydian".to_string(),
                "Mixolydian".to_string(),
            ],
        }
    }

    pub fn chromatic() -> Self {
        Self {
            name: "Chromatic".to_string(),
            intervals: (0..12).collect(),
            mode_names: vec!["Chromatic".to_string()],
        }
    }
}

// Add rand crate for humanization
// Use external `rand` crate's `random::<T>()` for humanization jitter