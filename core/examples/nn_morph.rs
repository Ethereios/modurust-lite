//! Complex Neural Network Latent Morphing Demo
//!
//! Demonstrates the handcrafted complex-valued network driving a 2D latent
//! space where phase dictates modulation trajectory and magnitude dictates
//! modulation depth, plus phase-preserving activations (modReLU / cardioid).
//!
//! Run: `cargo run -p modurust_core --example nn_morph`

use modurust_core::nn::{ComplexActivation, ComplexNeuralNetwork, NnMode};
use num_complex::Complex;

fn main() {
    println!("🧠 Modurust-Lite Complex Neural Network Latent Morphing Demo");

    let mut nn = ComplexNeuralNetwork::new(4, 8, 2);
    nn.set_mode(NnMode::Latent);
    nn.set_layer1_activation(ComplexActivation::ModRelu);
    nn.set_layer2_activation(ComplexActivation::Cardioid);

    // Sweep a 2D input plane: x in [-1, 1], y in [-1, 1].
    println!("\nMapping input (x, y) -> z (mag, phase):");
    for x in [-1.0f32, -0.5, 0.0, 0.5, 1.0] {
        for y in [-1.0f32, -0.5, 0.0, 0.5, 1.0] {
            let input = vec![
                Complex::new(x, 0.0),
                Complex::new(y, 0.0),
                Complex::new(0.2, 0.1),
                Complex::new(0.3, 0.0),
            ];
            let z = nn.map_to_complex_plane(&input);
            println!(
                "  x={x:>+5.1} y={y:>+5.1} -> z = {z:>7.3}  mag={:>6.3} phase={:>6.3} rad",
                z.norm(),
                z.arg()
            );
        }
    }

    // Demonstrate complex_phase mode for spectral/timbral morphing.
    nn.set_mode(NnMode::ComplexPhase);
    let input = vec![
        Complex::new(0.5, 0.0),
        Complex::new(0.5, 0.5),
        Complex::new(0.1, 0.2),
        Complex::new(0.3, 0.4),
    ];
    let out = nn.process_latent(&input);
    println!("\ncomplex_phase latent output (length {}):", out.len());
    for (i, z) in out.iter().enumerate() {
        println!("  out[{i}] = {z:>7.3}  mag={:.3} phase={:.3}", z.norm(), z.arg());
    }

    println!("\n✅ NN morph demo complete.");
}

