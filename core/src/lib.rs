pub mod dsp;
pub mod instance;
pub mod prime;
pub mod nn;
pub mod io;
pub mod comm;
pub mod config;
pub mod engine;

pub use config::EngineConfig;
pub use engine::{CoreEngine, EngineCommand, EngineMode, Param};

#[derive(Debug)]
pub struct ModurustCore {
    pub config: EngineConfig,
}

impl ModurustCore {
    pub fn new(config: EngineConfig) -> Self {
        Self { config }
    }
}
