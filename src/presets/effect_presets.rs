//! Effect presets - Reverb, delay, chorus, distortion

use super::{NodePreset, PresetCategory};

pub fn get_all() -> Vec<NodePreset> {
    vec![
        NodePreset::simple("Algorithmic Reverb", "Classic algorithmic reverb with adjustable room size", PresetCategory::Effects, "effect.reverb")
            .with_tags(vec!["reverb", "space", "room", "ambience"]),
        
        NodePreset::simple("Stereo Delay", "Stereo delay with feedback and filtering", PresetCategory::Effects, "effect.delay")
            .with_tags(vec!["delay", "echo", "feedback", "stereo"]),
        
        NodePreset::simple("Chorus", "Rich chorus effect for width and movement", PresetCategory::Effects, "effect.chorus")
            .with_tags(vec!["chorus", "modulation", "width", "ensemble"]),
        
        NodePreset::simple("Distortion", "Waveshaping distortion with tone control", PresetCategory::Effects, "effect.distortion")
            .with_tags(vec!["distortion", "overdrive", "saturation", "grit"]),
        
        NodePreset::simple("Phaser", "Sweeping phaser effect for movement", PresetCategory::Effects, "effect.phaser")
            .with_tags(vec!["phaser", "sweep", "modulation", "psychedelic"]),
        
        NodePreset::simple("Flanger", "Jet-like flanging effect", PresetCategory::Effects, "effect.flanger")
            .with_tags(vec!["flanger", "sweep", "jet", "metallic"]),
    ]
}
