use num_complex::Complex;

/// Activation function for complex-valued neurons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexActivation {
    /// Phase-preserving modReLU: `z * (|z| / (1 + |z|))`
    ModRelu,
    /// Cardioid activation (neural signal processing): preserves phase,
    /// scales magnitude by `0.5 * (1 + cos(arg))`.
    Cardioid,
    /// Pure linear (no activation).
    Linear,
}

impl ComplexActivation {
    pub fn apply(&self, z: Complex<f32>) -> Complex<f32> {
        match self {
            ComplexActivation::ModRelu => {
                let norm = z.norm();
                if norm > 0.0 {
                    z * (norm / (1.0 + norm))
                } else {
                    z
                }
            }
            ComplexActivation::Cardioid => {
                let norm = z.norm();
                if norm > 0.0 {
                    let arg = z.arg();
                    z * (0.5 * (1.0 + arg.cos()))
                } else {
                    z
                }
            }
            ComplexActivation::Linear => z,
        }
    }
}

pub struct ComplexLayer {
    pub weights: Vec<Vec<Complex<f32>>>,
    pub biases: Vec<Complex<f32>>,
    pub activation: ComplexActivation,
}

impl ComplexLayer {
    pub fn new(in_features: usize, out_features: usize) -> Self {
        let weights = vec![vec![Complex::new(0.1, 0.0); in_features]; out_features];
        let biases = vec![Complex::new(0.0, 0.0); out_features];
        Self {
            weights,
            biases,
            activation: ComplexActivation::ModRelu,
        }
    }

    pub fn with_activation(mut self, activation: ComplexActivation) -> Self {
        self.activation = activation;
        self
    }

    pub fn forward(&self, input: &[Complex<f32>]) -> Vec<Complex<f32>> {
        let mut output = vec![Complex::new(0.0, 0.0); self.weights.len()];
        for (i, row) in self.weights.iter().enumerate() {
            let mut sum = self.biases[i];
            for (j, w) in row.iter().enumerate() {
                if j < input.len() {
                    sum += w * input[j];
                }
            }
            output[i] = self.activation.apply(sum);
        }
        output
    }
}

/// Handcrafted complex-valued neural network.
///
/// Supports two inference modes:
/// - `complex_phase`: latent vector is passed through the network for spectral
///   / timbral morphing (phase-preserving activations).
/// - `latent`: mapping of audio/MIDI input onto a 2D complex plane where phase
///   dictates modulation trajectory and magnitude dictates modulation depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NnMode {
    ComplexPhase,
    Latent,
}

impl NnMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "latent" => NnMode::Latent,
            _ => NnMode::ComplexPhase,
        }
    }
}

pub struct ComplexNeuralNetwork {
    pub layer1: ComplexLayer,
    pub layer2: ComplexLayer,
    pub mode: NnMode,
}

impl ComplexNeuralNetwork {
    pub fn new(in_dim: usize, hidden_dim: usize, out_dim: usize) -> Self {
        Self {
            layer1: ComplexLayer::new(in_dim, hidden_dim),
            layer2: ComplexLayer::new(hidden_dim, out_dim),
            mode: NnMode::ComplexPhase,
        }
    }

    pub fn set_mode(&mut self, mode: NnMode) {
        self.mode = mode;
    }

    pub fn set_layer1_activation(&mut self, activation: ComplexActivation) {
        self.layer1.activation = activation;
    }

    pub fn set_layer2_activation(&mut self, activation: ComplexActivation) {
        self.layer2.activation = activation;
    }

    /// Run the network on a latent vector.
    pub fn process_latent(&self, input: &[Complex<f32>]) -> Vec<Complex<f32>> {
        let h = self.layer1.forward(input);
        self.layer2.forward(&h)
    }

    /// 2D complex-plane modulation mapping.
    ///
    /// Given an input vector (e.g. `[amplitude, frequency, x, y]`), returns
    /// a complex point `z`. The phase of `z` represents the modulation
    /// trajectory and the magnitude represents the modulation depth.
    pub fn map_to_complex_plane(&self, input: &[Complex<f32>]) -> Complex<f32> {
        let out = self.process_latent(input);
        out.first().copied().unwrap_or(Complex::new(0.0, 0.0))
    }
}

/// Feature-gated ONNX model loading (enabled with the `onnx` feature).
#[cfg(feature = "onnx")]
pub mod onnx {
    use super::*;

    /// A loaded ONNX model wrapper.
    pub struct OnnxModel {
        session: ort::Session,
    }

    impl OnnxModel {
        /// Load an ONNX model from disk.
        pub fn load(path: &str) -> Result<Self, String> {
            let session = ort::Session::builder()
                .and_then(|b| b.commit_from_file(path))
                .map_err(|e| format!("failed to load ONNX model: {e}"))?;
            Ok(Self { session })
        }

        /// Run inference on a complex-valued latent vector.
        ///
        /// The complex vector is encoded as a real tensor of shape
        /// `[1, in_features]` (magnitude/phase interleaved as [re, im]).
        pub fn infer(&self, input: &[Complex<f32>]) -> Result<Vec<Complex<f32>>, String> {
            // Flatten [re, im] pairs into a single real tensor.
            let mut flat = Vec::with_capacity(input.len() * 2);
            for c in input {
                flat.push(c.re);
                flat.push(c.im);
            }

            use ndarray::Array2;
            let arr = Array2::from_shape_vec((1, flat.len()), flat)
                .map_err(|e| format!("failed to build input tensor: {e}"))?;

            let outputs = self
                .session
                .run(ort::inputs!["input" => arr]?)
                .map_err(|e| format!("inference failed: {e}"))?;

            // Extract first output tensor and reshape back to complex pairs.
            let first = outputs[0].try_extract_tensor::<f32>()
                .map_err(|e| format!("failed to extract output: {e}"))?;

            let values: Vec<f32> = first.iter().copied().collect();
            let mut result = Vec::with_capacity(values.len() / 2);
            for chunk in values.chunks_exact(2) {
                result.push(Complex::new(chunk[0], chunk[1]));
            }
            Ok(result)
        }
    }

    /// Feature-gated loading helper: returns `Ok(Some(model))` when the `onnx`
    /// feature is enabled, otherwise `Ok(None)`.
    pub fn try_load_onnx(path: &str) -> Result<Option<OnnxModel>, String> {
        match OnnxModel::load(path) {
            Ok(model) => Ok(Some(model)),
            Err(e) => Err(e),
        }
    }
}

/// Non-ONNX fallback (when the `onnx` feature is disabled).
#[cfg(not(feature = "onnx"))]
pub mod onnx {
    /// Stub model type.
    pub struct OnnxModel;

    /// Always returns `Ok(None)` when the ONNX feature is not compiled in.
    pub fn try_load_onnx(_path: &str) -> Result<Option<OnnxModel>, String> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complex_network_shapes() {
        let nn = ComplexNeuralNetwork::new(4, 8, 4);
        let input = vec![
            Complex::new(0.1, 0.0),
            Complex::new(0.2, 0.1),
            Complex::new(0.3, 0.0),
            Complex::new(0.4, 0.2),
        ];
        let out = nn.process_latent(&input);
        assert_eq!(out.len(), 4);
    }

    #[test]
    fn test_mod_relu_preserves_phase() {
        let act = ComplexActivation::ModRelu;
        let z = Complex::new(1.0, 1.0);
        let out = act.apply(z);
        let phase_diff = (z.arg() - out.arg()).abs();
        assert!(phase_diff < 1e-3, "modReLU should preserve phase");
    }

    #[test]
    fn test_cardioid() {
        let act = ComplexActivation::Cardioid;
        let z = Complex::new(1.0, 0.0); // arg = 0
        let out = act.apply(z);
        // cos(0) = 1 -> scale 0.5 * 2 = 1.0
        assert!((out.norm() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn test_nn_mode_from_str() {
        assert_eq!(NnMode::from_str("complex_phase"), NnMode::ComplexPhase);
        assert_eq!(NnMode::from_str("latent"), NnMode::Latent);
    }
}

