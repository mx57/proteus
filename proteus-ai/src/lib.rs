//! # Proteus AI Core
//!
//! Core AI modules for Proteus — Thompson Sampling bandit, genetic evolution,
//! network fingerprinting, Wilson Score statistics.

pub mod bandit;
pub mod error;
pub mod evolver;
pub mod fingerprint;
pub mod genome;
pub mod history;
pub mod orchestrator;
pub mod registry;
pub mod signature;
pub mod wilson;

// Re-exports
pub use error::AiError;
pub use fingerprint::{FingerprintProvider, NetworkFingerprint};
pub use genome::{DpiEngineType, StrategyGenome, StrategyOrigin};
pub use history::{AiHistoryStore, HistoryRecord};
pub use orchestrator::{
    AiOrchestratorService, OrchestratorConfig, OrchestratorState, VerificationResult,
};
pub use signature::{compute as genome_signature, exists_in as genome_sig_exists};
