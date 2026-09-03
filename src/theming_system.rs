use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use egui::Color32;

/// Comprehensive theming system inspired by Ableton Live and Bitwig Studio
/// Provides extensive customization for colors, fonts, and visual elements

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeManager {
    pub current_theme: String,
    pub themes: HashMap<String, Theme>,
    pub color_palettes: HashMap<String, ColorPalette>,
    pub font_settings: FontSettings,
    pub user_themes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub colors: ThemeColors,
    pub typography: Typography,
    pub spacing: Spacing,
    pub borders: BorderSettings,
    pub shadows: ShadowSettings,
    pub animations: AnimationSettings,
    pub channel_colors: ChannelColorScheme,
    pub sample_colors: SampleColorScheme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    // Main interface colors
    pub background_primary: ColorRgba,
    pub background_secondary: ColorRgba,
    pub background_tertiary: ColorRgba,
    pub surface_primary: ColorRgba,
    pub surface_secondary: ColorRgba,
    pub surface_elevated: ColorRgba,
    
    // Text colors
    pub text_primary: ColorRgba,
    pub text_secondary: ColorRgba,
    pub text_disabled: ColorRgba,
    pub text_accent: ColorRgba,
    
    // Accent colors
    pub accent_primary: ColorRgba,
    pub accent_secondary: ColorRgba,
    pub accent_success: ColorRgba,
    pub accent_warning: ColorRgba,
    pub accent_error: ColorRgba,
    pub accent_info: ColorRgba,
    
    // Interactive elements
    pub button_primary: ColorRgba,
    pub button_secondary: ColorRgba,
    pub button_hover: ColorRgba,
    pub button_pressed: ColorRgba,
    pub button_disabled: ColorRgba,
    
    // Transport controls
    pub play_button: ColorRgba,
    pub stop_button: ColorRgba,
    pub record_button: ColorRgba,
    pub loop_button: ColorRgba,
    
    // Mixer elements
    pub fader_track: ColorRgba,
    pub fader_thumb: ColorRgba,
    pub knob_base: ColorRgba,
    pub knob_indicator: ColorRgba,
    pub vu_meter_green: ColorRgba,
    pub vu_meter_yellow: ColorRgba,
    pub vu_meter_red: ColorRgba,
    
    // Waveform colors
    pub waveform_positive: ColorRgba,
    pub waveform_negative: ColorRgba,
    pub waveform_background: ColorRgba,
    pub waveform_selection: ColorRgba,
    
    // Grid and timeline
    pub grid_major: ColorRgba,
    pub grid_minor: ColorRgba,
    pub timeline_background: ColorRgba,
    pub timeline_ruler: ColorRgba,
    pub playhead: ColorRgba,
    
    // Selection and focus
    pub selection_primary: ColorRgba,
    pub selection_secondary: ColorRgba,
    pub focus_ring: ColorRgba,
    pub drag_preview: ColorRgba,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelColorScheme {
    pub default_colors: Vec<ColorRgba>,
    pub instrument_colors: HashMap<String, ColorRgba>,
    pub category_colors: HashMap<ChannelCategory, ColorRgba>,
    pub state_colors: ChannelStateColors,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum ChannelCategory {
    Drums,
    Bass,
    Lead,
    Pad,
    Arp,
    Vocal,
    FX,
    Utility,
    Bus,
    Return,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStateColors {
    pub muted: ColorRgba,
    pub soloed: ColorRgba,
    pub recording: ColorRgba,
    pub armed: ColorRgba,
    pub playing: ColorRgba,
    pub selected: ColorRgba,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleColorScheme {
    pub by_type: HashMap<SampleType, ColorRgba>,
    pub by_key: HashMap<String, ColorRgba>, // Musical keys
    pub by_tempo: Vec<(f32, ColorRgba)>, // BPM ranges
    pub by_genre: HashMap<String, ColorRgba>,
    pub by_energy: Vec<(f32, ColorRgba)>, // Energy levels 0-1
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum SampleType {
    Kick,
    Snare,
    HiHat,
    Cymbal,
    Tom,
    Percussion,
    Bass,
    Lead,
    Pad,
    Vocal,
    FX,
    Loop,
    OneShot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPalette {
    pub name: String,
    pub colors: Vec<ColorRgba>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Typography {
    pub font_families: FontFamilies,
    pub font_sizes: FontSizes,
    pub line_heights: LineHeights,
    pub font_weights: FontWeights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontFamilies {
    pub primary: String,      // Main UI font
    pub secondary: String,    // Secondary text
    pub monospace: String,    // Code/numbers
    pub display: String,      // Headers/titles
    pub icon: String,         // Icon font
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSizes {
    pub xs: f32,      // 10px
    pub sm: f32,      // 12px
    pub base: f32,    // 14px
    pub lg: f32,      // 16px
    pub xl: f32,      // 18px
    pub xl2: f32,     // 20px
    pub xl3: f32,     // 24px
    pub xl4: f32,     // 32px
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineHeights {
    pub tight: f32,   // 1.2
    pub normal: f32,  // 1.4
    pub relaxed: f32, // 1.6
    pub loose: f32,   // 1.8
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontWeights {
    pub thin: u16,      // 100
    pub light: u16,     // 300
    pub normal: u16,    // 400
    pub medium: u16,    // 500
    pub semibold: u16,  // 600
    pub bold: u16,      // 700
    pub extrabold: u16, // 800
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSettings {
    pub available_fonts: Vec<String>,
    pub custom_fonts: HashMap<String, String>, // name -> path
    pub font_scaling: f32,
    pub subpixel_rendering: bool,
    pub hinting: FontHinting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FontHinting {
    None,
    Light,
    Normal,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spacing {
    pub xs: f32,    // 2px
    pub sm: f32,    // 4px
    pub base: f32,  // 8px
    pub lg: f32,    // 12px
    pub xl: f32,    // 16px
    pub xl2: f32,   // 24px
    pub xl3: f32,   // 32px
    pub xl4: f32,   // 48px
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorderSettings {
    pub width_thin: f32,
    pub width_normal: f32,
    pub width_thick: f32,
    pub radius_sm: f32,
    pub radius_base: f32,
    pub radius_lg: f32,
    pub radius_full: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowSettings {
    pub drop_shadow_sm: Shadow,
    pub drop_shadow_base: Shadow,
    pub drop_shadow_lg: Shadow,
    pub inner_shadow: Shadow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: ColorRgba,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationSettings {
    pub duration_fast: f32,     // 150ms
    pub duration_normal: f32,   // 300ms
    pub duration_slow: f32,     // 500ms
    pub easing: EasingFunction,
    pub enable_animations: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EasingFunction {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bounce,
    Elastic,
}

impl ThemeManager {
    pub fn new() -> Self {
        let mut manager = Self {
            current_theme: "Dark Professional".to_string(),
            themes: HashMap::new(),
            color_palettes: HashMap::new(),
            font_settings: FontSettings::default(),
            user_themes: Vec::new(),
        };
        
        manager.initialize_default_themes();
        manager.initialize_color_palettes();
        manager
    }

    fn initialize_default_themes(&mut self) {
        // Dark Professional Theme (Ableton-inspired)
        self.add_theme(Theme {
            name: "Dark Professional".to_string(),
            description: "Professional dark theme inspired by Ableton Live".to_string(),
            author: "Modurust".to_string(),
            version: "1.0".to_string(),
            colors: ThemeColors::dark_professional(),
            typography: Typography::default(),
            spacing: Spacing::default(),
            borders: BorderSettings::default(),
            shadows: ShadowSettings::default(),
            animations: AnimationSettings::default(),
            channel_colors: ChannelColorScheme::ableton_style(),
            sample_colors: SampleColorScheme::default(),
        });

        // Light Studio Theme (Bitwig-inspired)
        self.add_theme(Theme {
            name: "Light Studio".to_string(),
            description: "Clean light theme inspired by Bitwig Studio".to_string(),
            author: "Modurust".to_string(),
            version: "1.0".to_string(),
            colors: ThemeColors::light_studio(),
            typography: Typography::default(),
            spacing: Spacing::default(),
            borders: BorderSettings::default(),
            shadows: ShadowSettings::default(),
            animations: AnimationSettings::default(),
            channel_colors: ChannelColorScheme::bitwig_style(),
            sample_colors: SampleColorScheme::default(),
        });

        // Neon Theme
        self.add_theme(Theme {
            name: "Neon".to_string(),
            description: "Vibrant neon theme for creative sessions".to_string(),
            author: "Modurust".to_string(),
            version: "1.0".to_string(),
            colors: ThemeColors::neon(),
            typography: Typography::default(),
            spacing: Spacing::default(),
            borders: BorderSettings::default(),
            shadows: ShadowSettings::default(),
            animations: AnimationSettings::default(),
            channel_colors: ChannelColorScheme::neon_style(),
            sample_colors: SampleColorScheme::default(),
        });
    }

    fn initialize_color_palettes(&mut self) {
        // Ableton Live color palette
        self.color_palettes.insert("Ableton".to_string(), ColorPalette {
            name: "Ableton Live".to_string(),
            description: "Official Ableton Live color palette".to_string(),
            colors: vec![
                ColorRgba::from_hex("#FF6B35"), // Orange
                ColorRgba::from_hex("#F7931E"), // Yellow-Orange
                ColorRgba::from_hex("#FFD23F"), // Yellow
                ColorRgba::from_hex("#A8E6CF"), // Light Green
                ColorRgba::from_hex("#7FCDCD"), // Teal
                ColorRgba::from_hex("#7FB3D3"), // Light Blue
                ColorRgba::from_hex("#C7CEEA"), // Lavender
                ColorRgba::from_hex("#FFB7B2"), // Pink
                ColorRgba::from_hex("#FFDAC1"), // Peach
                ColorRgba::from_hex("#E2F0CB"), // Light Lime
                ColorRgba::from_hex("#B5EAD7"), // Mint
                ColorRgba::from_hex("#C7CEEA"), // Purple
            ],
        });

        // Bitwig Studio color palette
        self.color_palettes.insert("Bitwig".to_string(), ColorPalette {
            name: "Bitwig Studio".to_string(),
            description: "Bitwig Studio inspired colors".to_string(),
            colors: vec![
                ColorRgba::from_hex("#FF4444"), // Red
                ColorRgba::from_hex("#FF8844"), // Orange
                ColorRgba::from_hex("#FFDD44"), // Yellow
                ColorRgba::from_hex("#88FF44"), // Lime
                ColorRgba::from_hex("#44FF88"), // Green
                ColorRgba::from_hex("#44FFDD"), // Cyan
                ColorRgba::from_hex("#4488FF"), // Blue
                ColorRgba::from_hex("#8844FF"), // Purple
                ColorRgba::from_hex("#DD44FF"), // Magenta
                ColorRgba::from_hex("#FF44DD"), // Pink
            ],
        });
    }

    pub fn add_theme(&mut self, theme: Theme) {
        let name = theme.name.clone();
        self.themes.insert(name, theme);
    }

    pub fn set_current_theme(&mut self, theme_name: &str) -> Result<(), String> {
        if self.themes.contains_key(theme_name) {
            self.current_theme = theme_name.to_string();
            Ok(())
        } else {
            Err(format!("Theme '{}' not found", theme_name))
        }
    }

    pub fn get_current_theme(&self) -> Option<&Theme> {
        self.themes.get(&self.current_theme)
    }

    pub fn get_current_theme_mut(&mut self) -> Option<&mut Theme> {
        self.themes.get_mut(&self.current_theme)
    }

    pub fn get_channel_color(&self, channel_index: usize, category: Option<ChannelCategory>) -> ColorRgba {
        if let Some(theme) = self.get_current_theme() {
            if let Some(category) = category {
                if let Some(color) = theme.channel_colors.category_colors.get(&category) {
                    return color.clone();
                }
            }
            
            let colors = &theme.channel_colors.default_colors;
            if !colors.is_empty() {
                return colors[channel_index % colors.len()].clone();
            }
        }
        
        ColorRgba::from_hex("#888888") // Fallback gray
    }

    pub fn get_sample_color(&self, sample_type: SampleType) -> ColorRgba {
        if let Some(theme) = self.get_current_theme() {
            if let Some(color) = theme.sample_colors.by_type.get(&sample_type) {
                return color.clone();
            }
        }
        
        ColorRgba::from_hex("#666666") // Fallback gray
    }

    pub fn save_theme(&self, theme_name: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(theme) = self.themes.get(theme_name) {
            let json = serde_json::to_string_pretty(theme)?;
            std::fs::write(path, json)?;
            Ok(())
        } else {
            Err(format!("Theme '{}' not found", theme_name).into())
        }
    }

    pub fn load_theme(&mut self, path: &str) -> Result<String, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let theme: Theme = serde_json::from_str(&json)?;
        let theme_name = theme.name.clone();
        self.themes.insert(theme_name.clone(), theme);
        self.user_themes.push(theme_name.clone());
        Ok(theme_name)
    }
}

impl ColorRgba {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_hex(hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
        Self::new(r, g, b, 1.0)
    }

    pub fn to_color32(&self) -> Color32 {
        Color32::from_rgba_premultiplied(
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }
}

impl ThemeColors {
    pub fn dark_professional() -> Self {
        Self {
            background_primary: ColorRgba::from_hex("#1E1E1E"),
            background_secondary: ColorRgba::from_hex("#252526"),
            background_tertiary: ColorRgba::from_hex("#2D2D30"),
            surface_primary: ColorRgba::from_hex("#3C3C3C"),
            surface_secondary: ColorRgba::from_hex("#404040"),
            surface_elevated: ColorRgba::from_hex("#4A4A4A"),
            
            text_primary: ColorRgba::from_hex("#CCCCCC"),
            text_secondary: ColorRgba::from_hex("#969696"),
            text_disabled: ColorRgba::from_hex("#656565"),
            text_accent: ColorRgba::from_hex("#4FC3F7"),
            
            accent_primary: ColorRgba::from_hex("#007ACC"),
            accent_secondary: ColorRgba::from_hex("#4FC3F7"),
            accent_success: ColorRgba::from_hex("#4CAF50"),
            accent_warning: ColorRgba::from_hex("#FF9800"),
            accent_error: ColorRgba::from_hex("#F44336"),
            accent_info: ColorRgba::from_hex("#2196F3"),
            
            button_primary: ColorRgba::from_hex("#0E639C"),
            button_secondary: ColorRgba::from_hex("#3C3C3C"),
            button_hover: ColorRgba::from_hex("#1177BB"),
            button_pressed: ColorRgba::from_hex("#094771"),
            button_disabled: ColorRgba::from_hex("#2D2D30"),
            
            play_button: ColorRgba::from_hex("#4CAF50"),
            stop_button: ColorRgba::from_hex("#F44336"),
            record_button: ColorRgba::from_hex("#FF5722"),
            loop_button: ColorRgba::from_hex("#FF9800"),
            
            fader_track: ColorRgba::from_hex("#404040"),
            fader_thumb: ColorRgba::from_hex("#007ACC"),
            knob_base: ColorRgba::from_hex("#3C3C3C"),
            knob_indicator: ColorRgba::from_hex("#4FC3F7"),
            vu_meter_green: ColorRgba::from_hex("#4CAF50"),
            vu_meter_yellow: ColorRgba::from_hex("#FFEB3B"),
            vu_meter_red: ColorRgba::from_hex("#F44336"),
            
            waveform_positive: ColorRgba::from_hex("#4FC3F7"),
            waveform_negative: ColorRgba::from_hex("#29B6F6"),
            waveform_background: ColorRgba::from_hex("#1E1E1E"),
            waveform_selection: ColorRgba::from_hex("#007ACC"),
            
            grid_major: ColorRgba::from_hex("#404040"),
            grid_minor: ColorRgba::from_hex("#2D2D30"),
            timeline_background: ColorRgba::from_hex("#252526"),
            timeline_ruler: ColorRgba::from_hex("#3C3C3C"),
            playhead: ColorRgba::from_hex("#FF5722"),
            
            selection_primary: ColorRgba::from_hex("#264F78"),
            selection_secondary: ColorRgba::from_hex("#094771"),
            focus_ring: ColorRgba::from_hex("#007ACC"),
            drag_preview: ColorRgba::new(0.0, 0.48, 0.8, 0.3),
        }
    }

    pub fn light_studio() -> Self {
        Self {
            background_primary: ColorRgba::from_hex("#FFFFFF"),
            background_secondary: ColorRgba::from_hex("#F8F8F8"),
            background_tertiary: ColorRgba::from_hex("#F0F0F0"),
            surface_primary: ColorRgba::from_hex("#E8E8E8"),
            surface_secondary: ColorRgba::from_hex("#E0E0E0"),
            surface_elevated: ColorRgba::from_hex("#D8D8D8"),
            
            text_primary: ColorRgba::from_hex("#212121"),
            text_secondary: ColorRgba::from_hex("#757575"),
            text_disabled: ColorRgba::from_hex("#BDBDBD"),
            text_accent: ColorRgba::from_hex("#1976D2"),
            
            accent_primary: ColorRgba::from_hex("#1976D2"),
            accent_secondary: ColorRgba::from_hex("#42A5F5"),
            accent_success: ColorRgba::from_hex("#388E3C"),
            accent_warning: ColorRgba::from_hex("#F57C00"),
            accent_error: ColorRgba::from_hex("#D32F2F"),
            accent_info: ColorRgba::from_hex("#1976D2"),
            
            button_primary: ColorRgba::from_hex("#1976D2"),
            button_secondary: ColorRgba::from_hex("#E0E0E0"),
            button_hover: ColorRgba::from_hex("#1565C0"),
            button_pressed: ColorRgba::from_hex("#0D47A1"),
            button_disabled: ColorRgba::from_hex("#F5F5F5"),
            
            play_button: ColorRgba::from_hex("#388E3C"),
            stop_button: ColorRgba::from_hex("#D32F2F"),
            record_button: ColorRgba::from_hex("#E64A19"),
            loop_button: ColorRgba::from_hex("#F57C00"),
            
            fader_track: ColorRgba::from_hex("#E0E0E0"),
            fader_thumb: ColorRgba::from_hex("#1976D2"),
            knob_base: ColorRgba::from_hex("#E8E8E8"),
            knob_indicator: ColorRgba::from_hex("#42A5F5"),
            vu_meter_green: ColorRgba::from_hex("#4CAF50"),
            vu_meter_yellow: ColorRgba::from_hex("#FFEB3B"),
            vu_meter_red: ColorRgba::from_hex("#F44336"),
            
            waveform_positive: ColorRgba::from_hex("#42A5F5"),
            waveform_negative: ColorRgba::from_hex("#1976D2"),
            waveform_background: ColorRgba::from_hex("#FFFFFF"),
            waveform_selection: ColorRgba::from_hex("#1976D2"),
            
            grid_major: ColorRgba::from_hex("#E0E0E0"),
            grid_minor: ColorRgba::from_hex("#F0F0F0"),
            timeline_background: ColorRgba::from_hex("#F8F8F8"),
            timeline_ruler: ColorRgba::from_hex("#E8E8E8"),
            playhead: ColorRgba::from_hex("#E64A19"),
            
            selection_primary: ColorRgba::from_hex("#BBDEFB"),
            selection_secondary: ColorRgba::from_hex("#90CAF9"),
            focus_ring: ColorRgba::from_hex("#1976D2"),
            drag_preview: ColorRgba::new(0.1, 0.46, 0.82, 0.3),
        }
    }

    pub fn neon() -> Self {
        Self {
            background_primary: ColorRgba::from_hex("#0A0A0A"),
            background_secondary: ColorRgba::from_hex("#1A1A1A"),
            background_tertiary: ColorRgba::from_hex("#2A2A2A"),
            surface_primary: ColorRgba::from_hex("#3A3A3A"),
            surface_secondary: ColorRgba::from_hex("#4A4A4A"),
            surface_elevated: ColorRgba::from_hex("#5A5A5A"),
            
            text_primary: ColorRgba::from_hex("#00FFFF"),
            text_secondary: ColorRgba::from_hex("#FF00FF"),
            text_disabled: ColorRgba::from_hex("#666666"),
            text_accent: ColorRgba::from_hex("#FFFF00"),
            
            accent_primary: ColorRgba::from_hex("#00FFFF"),
            accent_secondary: ColorRgba::from_hex("#FF00FF"),
            accent_success: ColorRgba::from_hex("#00FF00"),
            accent_warning: ColorRgba::from_hex("#FFFF00"),
            accent_error: ColorRgba::from_hex("#FF0080"),
            accent_info: ColorRgba::from_hex("#0080FF"),
            
            button_primary: ColorRgba::from_hex("#00FFFF"),
            button_secondary: ColorRgba::from_hex("#FF00FF"),
            button_hover: ColorRgba::from_hex("#80FFFF"),
            button_pressed: ColorRgba::from_hex("#008080"),
            button_disabled: ColorRgba::from_hex("#404040"),
            
            play_button: ColorRgba::from_hex("#00FF00"),
            stop_button: ColorRgba::from_hex("#FF0080"),
            record_button: ColorRgba::from_hex("#FF4000"),
            loop_button: ColorRgba::from_hex("#FFFF00"),
            
            fader_track: ColorRgba::from_hex("#404040"),
            fader_thumb: ColorRgba::from_hex("#00FFFF"),
            knob_base: ColorRgba::from_hex("#3A3A3A"),
            knob_indicator: ColorRgba::from_hex("#FF00FF"),
            vu_meter_green: ColorRgba::from_hex("#00FF00"),
            vu_meter_yellow: ColorRgba::from_hex("#FFFF00"),
            vu_meter_red: ColorRgba::from_hex("#FF0080"),
            
            waveform_positive: ColorRgba::from_hex("#00FFFF"),
            waveform_negative: ColorRgba::from_hex("#FF00FF"),
            waveform_background: ColorRgba::from_hex("#0A0A0A"),
            waveform_selection: ColorRgba::from_hex("#FFFF00"),
            
            grid_major: ColorRgba::from_hex("#404040"),
            grid_minor: ColorRgba::from_hex("#2A2A2A"),
            timeline_background: ColorRgba::from_hex("#1A1A1A"),
            timeline_ruler: ColorRgba::from_hex("#3A3A3A"),
            playhead: ColorRgba::from_hex("#FF4000"),
            
            selection_primary: ColorRgba::from_hex("#008080"),
            selection_secondary: ColorRgba::from_hex("#800080"),
            focus_ring: ColorRgba::from_hex("#00FFFF"),
            drag_preview: ColorRgba::new(0.0, 1.0, 1.0, 0.3),
        }
    }
}

impl ChannelColorScheme {
    pub fn ableton_style() -> Self {
        Self {
            default_colors: vec![
                ColorRgba::from_hex("#FF6B35"), // Orange
                ColorRgba::from_hex("#F7931E"), // Yellow-Orange
                ColorRgba::from_hex("#FFD23F"), // Yellow
                ColorRgba::from_hex("#A8E6CF"), // Light Green
                ColorRgba::from_hex("#7FCDCD"), // Teal
                ColorRgba::from_hex("#7FB3D3"), // Light Blue
                ColorRgba::from_hex("#C7CEEA"), // Lavender
                ColorRgba::from_hex("#FFB7B2"), // Pink
                ColorRgba::from_hex("#FFDAC1"), // Peach
                ColorRgba::from_hex("#E2F0CB"), // Light Lime
                ColorRgba::from_hex("#B5EAD7"), // Mint
                ColorRgba::from_hex("#C7CEEA"), // Purple
            ],
            instrument_colors: HashMap::new(),
            category_colors: {
                let mut colors = HashMap::new();
                colors.insert(ChannelCategory::Drums, ColorRgba::from_hex("#FF6B35"));
                colors.insert(ChannelCategory::Bass, ColorRgba::from_hex("#7FB3D3"));
                colors.insert(ChannelCategory::Lead, ColorRgba::from_hex("#FFD23F"));
                colors.insert(ChannelCategory::Pad, ColorRgba::from_hex("#A8E6CF"));
                colors.insert(ChannelCategory::Arp, ColorRgba::from_hex("#C7CEEA"));
                colors.insert(ChannelCategory::Vocal, ColorRgba::from_hex("#FFB7B2"));
                colors.insert(ChannelCategory::FX, ColorRgba::from_hex("#7FCDCD"));
                colors.insert(ChannelCategory::Utility, ColorRgba::from_hex("#E2F0CB"));
                colors.insert(ChannelCategory::Bus, ColorRgba::from_hex("#B5EAD7"));
                colors.insert(ChannelCategory::Return, ColorRgba::from_hex("#FFDAC1"));
                colors
            },
            state_colors: ChannelStateColors {
                muted: ColorRgba::from_hex("#666666"),
                soloed: ColorRgba::from_hex("#FFD700"),
                recording: ColorRgba::from_hex("#FF0000"),
                armed: ColorRgba::from_hex("#FF4500"),
                playing: ColorRgba::from_hex("#00FF00"),
                selected: ColorRgba::from_hex("#0080FF"),
            },
        }
    }

    pub fn bitwig_style() -> Self {
        Self {
            default_colors: vec![
                ColorRgba::from_hex("#FF4444"), // Red
                ColorRgba::from_hex("#FF8844"), // Orange
                ColorRgba::from_hex("#FFDD44"), // Yellow
                ColorRgba::from_hex("#88FF44"), // Lime
                ColorRgba::from_hex("#44FF88"), // Green
                ColorRgba::from_hex("#44FFDD"), // Cyan
                ColorRgba::from_hex("#4488FF"), // Blue
                ColorRgba::from_hex("#8844FF"), // Purple
                ColorRgba::from_hex("#DD44FF"), // Magenta
                ColorRgba::from_hex("#FF44DD"), // Pink
            ],
            instrument_colors: HashMap::new(),
            category_colors: {
                let mut colors = HashMap::new();
                colors.insert(ChannelCategory::Drums, ColorRgba::from_hex("#FF4444"));
                colors.insert(ChannelCategory::Bass, ColorRgba::from_hex("#4488FF"));
                colors.insert(ChannelCategory::Lead, ColorRgba::from_hex("#FFDD44"));
                colors.insert(ChannelCategory::Pad, ColorRgba::from_hex("#44FF88"));
                colors.insert(ChannelCategory::Arp, ColorRgba::from_hex("#8844FF"));
                colors.insert(ChannelCategory::Vocal, ColorRgba::from_hex("#FF44DD"));
                colors.insert(ChannelCategory::FX, ColorRgba::from_hex("#44FFDD"));
                colors.insert(ChannelCategory::Utility, ColorRgba::from_hex("#88FF44"));
                colors.insert(ChannelCategory::Bus, ColorRgba::from_hex("#DD44FF"));
                colors.insert(ChannelCategory::Return, ColorRgba::from_hex("#FF8844"));
                colors
            },
            state_colors: ChannelStateColors {
                muted: ColorRgba::from_hex("#666666"),
                soloed: ColorRgba::from_hex("#FFFF00"),
                recording: ColorRgba::from_hex("#FF0000"),
                armed: ColorRgba::from_hex("#FF6600"),
                playing: ColorRgba::from_hex("#00FF00"),
                selected: ColorRgba::from_hex("#00AAFF"),
            },
        }
    }

    pub fn neon_style() -> Self {
        Self {
            default_colors: vec![
                ColorRgba::from_hex("#00FFFF"), // Cyan
                ColorRgba::from_hex("#FF00FF"), // Magenta
                ColorRgba::from_hex("#FFFF00"), // Yellow
                ColorRgba::from_hex("#00FF00"), // Green
                ColorRgba::from_hex("#FF0080"), // Hot Pink
                ColorRgba::from_hex("#0080FF"), // Electric Blue
                ColorRgba::from_hex("#80FF00"), // Lime
                ColorRgba::from_hex("#FF8000"), // Orange
                ColorRgba::from_hex("#8000FF"), // Purple
                ColorRgba::from_hex("#FF0040"), // Red-Pink
            ],
            instrument_colors: HashMap::new(),
            category_colors: {
                let mut colors = HashMap::new();
                colors.insert(ChannelCategory::Drums, ColorRgba::from_hex("#FF0080"));
                colors.insert(ChannelCategory::Bass, ColorRgba::from_hex("#0080FF"));
                colors.insert(ChannelCategory::Lead, ColorRgba::from_hex("#FFFF00"));
                colors.insert(ChannelCategory::Pad, ColorRgba::from_hex("#00FF00"));
                colors.insert(ChannelCategory::Arp, ColorRgba::from_hex("#8000FF"));
                colors.insert(ChannelCategory::Vocal, ColorRgba::from_hex("#FF00FF"));
                colors.insert(ChannelCategory::FX, ColorRgba::from_hex("#00FFFF"));
                colors.insert(ChannelCategory::Utility, ColorRgba::from_hex("#80FF00"));
                colors.insert(ChannelCategory::Bus, ColorRgba::from_hex("#FF8000"));
                colors.insert(ChannelCategory::Return, ColorRgba::from_hex("#FF0040"));
                colors
            },
            state_colors: ChannelStateColors {
                muted: ColorRgba::from_hex("#404040"),
                soloed: ColorRgba::from_hex("#FFFF00"),
                recording: ColorRgba::from_hex("#FF0080"),
                armed: ColorRgba::from_hex("#FF4000"),
                playing: ColorRgba::from_hex("#00FF00"),
                selected: ColorRgba::from_hex("#00FFFF"),
            },
        }
    }
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            available_fonts: vec![
                "Inter".to_string(),
                "Roboto".to_string(),
                "Open Sans".to_string(),
                "Source Sans Pro".to_string(),
                "Lato".to_string(),
                "Montserrat".to_string(),
                "Poppins".to_string(),
                "Nunito".to_string(),
                "Fira Code".to_string(), // Monospace
                "JetBrains Mono".to_string(), // Monospace
            ],
            custom_fonts: HashMap::new(),
            font_scaling: 1.0,
            subpixel_rendering: true,
            hinting: FontHinting::Normal,
        }
    }
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            font_families: FontFamilies {
                primary: "Inter".to_string(),
                secondary: "Inter".to_string(),
                monospace: "Fira Code".to_string(),
                display: "Inter".to_string(),
                icon: "Material Icons".to_string(),
            },
            font_sizes: FontSizes {
                xs: 10.0,
                sm: 12.0,
                base: 14.0,
                lg: 16.0,
                xl: 18.0,
                xl2: 20.0,
                xl3: 24.0,
                xl4: 32.0,
            },
            line_heights: LineHeights {
                tight: 1.2,
                normal: 1.4,
                relaxed: 1.6,
                loose: 1.8,
            },
            font_weights: FontWeights {
                thin: 100,
                light: 300,
                normal: 400,
                medium: 500,
                semibold: 600,
                bold: 700,
                extrabold: 800,
            },
        }
    }
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            xs: 2.0,
            sm: 4.0,
            base: 8.0,
            lg: 12.0,
            xl: 16.0,
            xl2: 24.0,
            xl3: 32.0,
            xl4: 48.0,
        }
    }
}

impl Default for BorderSettings {
    fn default() -> Self {
        Self {
            width_thin: 1.0,
            width_normal: 2.0,
            width_thick: 4.0,
            radius_sm: 2.0,
            radius_base: 4.0,
            radius_lg: 8.0,
            radius_full: 9999.0,
        }
    }
}

impl Default for ShadowSettings {
    fn default() -> Self {
        Self {
            drop_shadow_sm: Shadow {
                offset_x: 0.0,
                offset_y: 1.0,
                blur: 2.0,
                spread: 0.0,
                color: ColorRgba::new(0.0, 0.0, 0.0, 0.1),
            },
            drop_shadow_base: Shadow {
                offset_x: 0.0,
                offset_y: 2.0,
                blur: 4.0,
                spread: 0.0,
                color: ColorRgba::new(0.0, 0.0, 0.0, 0.15),
            },
            drop_shadow_lg: Shadow {
                offset_x: 0.0,
                offset_y: 4.0,
                blur: 8.0,
                spread: 0.0,
                color: ColorRgba::new(0.0, 0.0, 0.0, 0.2),
            },
            inner_shadow: Shadow {
                offset_x: 0.0,
                offset_y: 1.0,
                blur: 2.0,
                spread: 0.0,
                color: ColorRgba::new(0.0, 0.0, 0.0, 0.05),
            },
        }
    }
}

impl Default for AnimationSettings {
    fn default() -> Self {
        Self {
            duration_fast: 0.15,
            duration_normal: 0.3,
            duration_slow: 0.5,
            easing: EasingFunction::EaseInOut,
            enable_animations: true,
        }
    }
}

impl Default for SampleColorScheme {
    fn default() -> Self {
        Self {
            by_type: {
                let mut colors = HashMap::new();
                colors.insert(SampleType::Kick, ColorRgba::from_hex("#FF4444"));
                colors.insert(SampleType::Snare, ColorRgba::from_hex("#FF8844"));
                colors.insert(SampleType::HiHat, ColorRgba::from_hex("#FFDD44"));
                colors.insert(SampleType::Cymbal, ColorRgba::from_hex("#88FF44"));
                colors.insert(SampleType::Tom, ColorRgba::from_hex("#44FF88"));
                colors.insert(SampleType::Percussion, ColorRgba::from_hex("#44FFDD"));
                colors.insert(SampleType::Bass, ColorRgba::from_hex("#4488FF"));
                colors.insert(SampleType::Lead, ColorRgba::from_hex("#8844FF"));
                colors.insert(SampleType::Pad, ColorRgba::from_hex("#DD44FF"));
                colors.insert(SampleType::Vocal, ColorRgba::from_hex("#FF44DD"));
                colors.insert(SampleType::FX, ColorRgba::from_hex("#44FF44"));
                colors.insert(SampleType::Loop, ColorRgba::from_hex("#FF4488"));
                colors.insert(SampleType::OneShot, ColorRgba::from_hex("#88FF88"));
                colors
            },
            by_key: HashMap::new(),
            by_tempo: vec![
                (80.0, ColorRgba::from_hex("#4444FF")),   // Slow
                (120.0, ColorRgba::from_hex("#44FF44")),  // Medium
                (140.0, ColorRgba::from_hex("#FFFF44")),  // Fast
                (180.0, ColorRgba::from_hex("#FF4444")),  // Very Fast
            ],
            by_genre: HashMap::new(),
            by_energy: vec![
                (0.3, ColorRgba::from_hex("#4444FF")),    // Low energy
                (0.6, ColorRgba::from_hex("#44FF44")),    // Medium energy
                (0.8, ColorRgba::from_hex("#FFFF44")),    // High energy
                (1.0, ColorRgba::from_hex("#FF4444")),    // Maximum energy
            ],
        }
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}