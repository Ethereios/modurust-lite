//! Utility presets - Mixers, splitters, VCAs, outputs

use super::{NodePreset, PresetCategory};

pub fn get_all() -> Vec<NodePreset> {
    vec![
        NodePreset::simple("4-Channel Mixer", "Mixer with 4 inputs and volume controls", PresetCategory::Utilities, "utility.mixer")
            .with_tags(vec!["mixer", "combine", "sum", "blend"]),
        
        NodePreset::simple("VCA", "Voltage controlled amplifier", PresetCategory::Utilities, "utility.vca")
            .with_tags(vec!["vca", "amplifier", "gain", "control"]),
        
        NodePreset::simple("Signal Splitter", "Split signal to multiple outputs", PresetCategory::Utilities, "utility.splitter")
            .with_tags(vec!["splitter", "mult", "duplicate", "copy"]),
        
        NodePreset::simple("Audio Output", "Final output node for audio", PresetCategory::Utilities, "utility.output")
            .with_tags(vec!["output", "master", "final", "speakers"]),
        
        NodePreset::simple("Panner", "Stereo panning control", PresetCategory::Utilities, "utility.panner")
            .with_tags(vec!["pan", "stereo", "position", "balance"]),
    ]
}
