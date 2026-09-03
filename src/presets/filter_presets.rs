//! Filter presets - LPF, HPF, BPF, Notch

use super::{NodePreset, PresetCategory};

pub fn get_all() -> Vec<NodePreset> {
    vec![
        NodePreset::simple("Low Pass Filter", "Classic low-pass filter for removing high frequencies", PresetCategory::Filters, "filter.lpf")
            .with_tags(vec!["filter", "lpf", "lowpass", "moog"]),
        
        NodePreset::simple("High Pass Filter", "High-pass filter for removing low frequencies", PresetCategory::Filters, "filter.hpf")
            .with_tags(vec!["filter", "hpf", "highpass", "rumble"]),
        
        NodePreset::simple("Band Pass Filter", "Band-pass filter for isolating frequency ranges", PresetCategory::Filters, "filter.bpf")
            .with_tags(vec!["filter", "bpf", "bandpass", "resonance"]),
        
        NodePreset::simple("Notch Filter", "Notch filter for removing specific frequencies", PresetCategory::Filters, "filter.notch")
            .with_tags(vec!["filter", "notch", "bandstop", "remove"]),
        
        NodePreset::simple("State Variable Filter", "Multimode filter with LP/HP/BP outputs", PresetCategory::Filters, "filter.svf")
            .with_tags(vec!["filter", "svf", "multimode", "versatile"]),
    ]
}
