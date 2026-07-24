//! # BSDPI AI Core
//!
//! Core AI modules for BSDPI — Thompson Sampling bandit, genetic evolution,
//! network fingerprinting, Wilson Score statistics.

pub mod wilson;
pub mod fingerprint;
pub mod genome;
pub mod signature;
pub mod bandit;
pub mod evolver;
pub mod registry;
pub mod history;
pub mod error;

// Re-exports
pub use error::AiError;
pub use fingerprint::{FingerprintProvider, NetworkFingerprint};
pub use genome::{DpiEngineType, StrategyGenome, StrategyOrigin};
pub use history::{HistoryRecord, AiHistoryStore};
pub use signature::{compute as genome_signature, exists_in as genome_sig_exists};

pub mod orchestrator;
pub use orchestrator::{AiOrchestratorService, OrchestratorState};
