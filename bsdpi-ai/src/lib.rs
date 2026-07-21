//! # BSDPI AI Core
//!
//! Ядро AI-системы BSDPI:
//! - **WilsonScore** — статистическая оценка качества стратегий
//! - **NetworkFingerprint** — идентификация сети по SHA256 слепку
//! - **StrategyGenome** — геном стратегии DPI-обхода (50+ параметров)
//! - **GenomeSignature** — уникальная сигнатура генома
//! - **BanditSelector** — Thompson Sampling + UCB1 bandit
//! - **StrategyEvolver** — генетическая эволюция стратегий
//! - **AiStrategyRegistry** — персистентное хранилище стратегий
//! - **AiHistoryStore** — лог истории работы
//! - **AiOrchestratorService** — конечный автомат оркестрации

pub mod wilson;
pub mod fingerprint;
pub mod genome;
pub mod signature;
pub mod bandit;
pub mod evolver;
pub mod registry;
pub mod history;
pub mod orchestrator;

// Re-exports
pub use wilson::WilsonScore;
pub use fingerprint::{NetworkFingerprint, FingerprintProvider, FingerprintProviderImpl};
pub use genome::{StrategyGenome, DpiEngineType, StrategyOrigin, EngineProfile};
pub use signature::GenomeSignature;
pub use bandit::{BanditSelector, BanditArm, BanditConfig};
pub use evolver::{StrategyEvolver, EvolutionConfig, EvolutionStats};
pub use registry::{AiStrategyRegistry, BanditStateEntry, RecordStatus};
pub use history::{AiHistoryStore, HistoryRecord, WorkEvent, WorkResult};
pub use orchestrator::{AiOrchestratorService, OrchestratorConfig, OrchestratorEvent, OrchestratorState};
