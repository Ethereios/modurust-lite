//! Generator presets - Oscillators, samplers, and noise generators

use super::{NodePreset, PresetCategory};

pub fn get_all() -> Vec<NodePreset> {
    vec![
        sine_oscillator(),
        saw_oscillator(),
        square_oscillator(),
        triangle_oscillator(),
        white_noise(),
        pink_noise(),
        sampler(),
        wavetable(),
    ]
}

fn sine_oscillator() -> NodePreset {
    NodePreset::simple(
        "Sine Oscillator",
        "Pure sine wave oscillator with adjustable frequency and amplitude",
        PresetCategory::Generators,
        "generator.sine",
    )
    .with_tags(vec!["oscillator", "sine", "basic", "tone"])
}

fn saw_oscillator() -> NodePreset {
    NodePreset::simple(
        "Sawtooth Oscillator",
        "Bright sawtooth wave with rich harmonics",
        PresetCategory::Generators,
        "generator.saw",
    )
    .with_tags(vec!["oscillator", "saw", "sawtooth", "bright"])
}

fn square_oscillator() -> NodePreset {
    NodePreset::simple(
        "Square Oscillator",
        "Hollow square wave with adjustable pulse width",
        PresetCategory::Generators,
        "generator.square",
    )
    .with_tags(vec!["oscillator", "square", "pulse", "hollow"])
}

fn triangle_oscillator() -> NodePreset {
    NodePreset::simple(
        "Triangle Oscillator",
        "Smooth triangle wave with softer harmonics",
        PresetCategory::Generators,
        "generator.triangle",
    )
    .with_tags(vec!["oscillator", "triangle", "smooth", "soft"])
}

fn white_noise() -> NodePreset {
    NodePreset::simple(
        "White Noise",
        "Full-spectrum white noise generator",
        PresetCategory::Generators,
        "generator.noise.white",
    )
    .with_tags(vec!["noise", "white", "random", "hiss"])
}

fn pink_noise() -> NodePreset {
    NodePreset::simple(
        "Pink Noise",
        "Equal energy per octave pink noise (1/f)",
        PresetCategory::Generators,
        "generator.noise.pink",
    )
    .with_tags(vec!["noise", "pink", "random", "natural"])
}

fn sampler() -> NodePreset {
    NodePreset::simple(
        "Audio Sampler",
        "Sample playback with pitch control and looping",
        PresetCategory::Generators,
        "generator.sampler",
    )
    .with_tags(vec!["sampler", "audio", "playback", "clip"])
}

fn wavetable() -> NodePreset {
    NodePreset::simple(
        "Wavetable Synth",
        "Wavetable synthesis with position morphing",
        PresetCategory::Generators,
        "generator.wavetable",
    )
    .with_tags(vec!["wavetable", "synth", "morph", "modern"])
}
