//! Cookbook presets - Complex modular synthesis examples from documentation

use super::{NodePreset, PresetCategory, PresetNode, PresetConnection};

pub fn get_all() -> Vec<NodePreset> {
    vec![
        resonant_filter_chain(),
        fm_synthesis(),
        wave_folder_envelope(),
        mixer_with_envelopes(),
        feedback_delay(),
    ]
}

/// Resonant Filter Chain (Cookbook p.60)
fn resonant_filter_chain() -> NodePreset {
    let nodes = vec![
        PresetNode::new("osc", "generator.saw", (0.0, 0.0))
            .with_parameter("frequency", 220.0, 20.0, 20000.0),
        PresetNode::new("lpf", "filter.lpf", (150.0, 0.0))
            .with_parameter("cutoff", 1000.0, 20.0, 20000.0)
            .with_parameter("resonance", 3.0, 0.1, 10.0),
        PresetNode::new("bpf", "filter.bpf", (300.0, 0.0))
            .with_parameter("cutoff", 2000.0, 20.0, 20000.0)
            .with_parameter("resonance", 2.0, 0.1, 10.0),
        PresetNode::new("hpf", "filter.hpf", (450.0, 0.0))
            .with_parameter("cutoff", 3000.0, 20.0, 20000.0)
            .with_parameter("resonance", 1.5, 0.1, 10.0),
        PresetNode::new("lfo", "modulator.lfo", (150.0, 100.0))
            .with_parameter("frequency", 0.5, 0.01, 100.0),
    ];

    let connections = vec![
        PresetConnection::new("osc", "out", "lpf", "in"),
        PresetConnection::new("lpf", "out", "bpf", "in"),
        PresetConnection::new("bpf", "out", "hpf", "in"),
        PresetConnection::new("lfo", "out", "lpf", "cutoff_mod"),
    ];

    NodePreset::complex(
        "Resonant Filter Chain",
        "Three cascaded filters with LFO modulation (Cookbook p.60)",
        PresetCategory::Cookbook,
        nodes,
        connections,
    )
    .with_tags(vec!["filter", "cascade", "resonance", "modulation"])
}

/// FM Synthesis (Cookbook p.55)
fn fm_synthesis() -> NodePreset {
    let nodes = vec![
        PresetNode::new("carrier", "generator.sine", (200.0, 0.0))
            .with_parameter("frequency", 440.0, 20.0, 20000.0),
        PresetNode::new("modulator", "generator.sine", (0.0, 0.0))
            .with_parameter("frequency", 880.0, 20.0, 20000.0)
            .with_parameter("amplitude", 200.0, 0.0, 1000.0),
        PresetNode::new("envelope", "modulator.envelope", (0.0, 100.0))
            .with_parameter("attack", 0.01, 0.0, 1.0)
            .with_parameter("decay", 0.2, 0.0, 1.0)
            .with_parameter("sustain", 0.5, 0.0, 1.0)
            .with_parameter("release", 0.3, 0.0, 5.0),
        PresetNode::new("vca", "utility.vca", (200.0, 100.0)),
    ];

    let connections = vec![
        PresetConnection::new("modulator", "out", "carrier", "freq_mod"),
        PresetConnection::new("carrier", "out", "vca", "in"),
        PresetConnection::new("envelope", "out", "vca", "cv"),
    ];

    NodePreset::complex(
        "FM Synthesis",
        "Classic frequency modulation synthesis (Cookbook p.55)",
        PresetCategory::Cookbook,
        nodes,
        connections,
    )
    .with_tags(vec!["fm", "synthesis", "modulation", "bell"])
}

/// Wave Folder + Envelope (Cookbook p.62)
fn wave_folder_envelope() -> NodePreset {
    let nodes = vec![
        PresetNode::new("osc", "generator.triangle", (0.0, 0.0))
            .with_parameter("frequency", 110.0, 20.0, 20000.0),
        PresetNode::new("wavefolder", "effect.wavefolder", (150.0, 0.0))
            .with_parameter("drive", 2.0, 1.0, 10.0),
        PresetNode::new("envelope", "modulator.envelope", (0.0, 100.0)),
        PresetNode::new("attenuator", "utility.vca", (150.0, 100.0)),
    ];

    let connections = vec![
        PresetConnection::new("osc", "out", "wavefolder", "in"),
        PresetConnection::new("envelope", "out", "wavefolder", "drive_mod"),
        PresetConnection::new("wavefolder", "out", "attenuator", "in"),
    ];

    NodePreset::complex(
        "Wave Folder + Envelope",
        "Wavef olding with envelope shaping (Cookbook p.62)",
        PresetCategory::Cookbook,
        nodes,
        connections,
    )
    .with_tags(vec!["wavefolder", "envelope", "distortion", "west_coast"])
}

/// Mixer with Envelopes (Cookbook p.64)
fn mixer_with_envelopes() -> NodePreset {
    let nodes = vec![
        PresetNode::new("osc1", "generator.sine", (0.0, 0.0)),
        PresetNode::new("osc2", "generator.saw", (0.0, 80.0)),
        PresetNode::new("env1", "modulator.envelope", (100.0, 0.0)),
        PresetNode::new("env2", "modulator.envelope", (100.0, 80.0)),
        PresetNode::new("vca1", "utility.vca", (200.0, 0.0)),
        PresetNode::new("vca2", "utility.vca", (200.0, 80.0)),
        PresetNode::new("mixer", "utility.mixer", (300.0, 40.0)),
    ];

    let connections = vec![
        PresetConnection::new("osc1", "out", "vca1", "in"),
        PresetConnection::new("osc2", "out", "vca2", "in"),
        PresetConnection::new("env1", "out", "vca1", "cv"),
        PresetConnection::new("env2", "out", "vca2", "cv"),
        PresetConnection::new("vca1", "out", "mixer", "in1"),
        PresetConnection::new("vca2", "out", "mixer", "in2"),
    ];

    NodePreset::complex(
        "Mixer with Envelopes",
        "4-channel mixer with independent envelope control (Cookbook p.64)",
        PresetCategory::Cookbook,
        nodes,
        connections,
    )
    .with_tags(vec!["mixer", "envelope", "vca", "polyphonic"])
}

/// Feedback Delay (Cookbook p.88)
fn feedback_delay() -> NodePreset {
    let nodes = vec![
        PresetNode::new("input", "utility.input", (0.0, 0.0)),
        PresetNode::new("delay", "effect.delay", (150.0, 0.0))
            .with_parameter("time", 0.25, 0.001, 2.0)
            .with_parameter("feedback", 0.6, 0.0, 0.99),
        PresetNode::new("filter", "filter.lpf", (300.0, 0.0))
            .with_parameter("cutoff", 2000.0, 20.0, 20000.0),
        PresetNode::new("feedback_vca", "utility.vca", (300.0, 80.0)),
    ];

    let connections = vec![
        PresetConnection::new("input", "out", "delay", "in"),
        PresetConnection::new("delay", "out", "filter", "in"),
        PresetConnection::new("filter", "out", "feedback_vca", "in"),
        PresetConnection::new("feedback_vca", "out", "delay", "feedback"),
    ];

    NodePreset::complex(
        "Feedback Delay",
        "Delay with filtered feedback path (Cookbook p.88)",
        PresetCategory::Cookbook,
        nodes,
        connections,
    )
    .with_tags(vec!["delay", "feedback", "filter", "dub"])
}
