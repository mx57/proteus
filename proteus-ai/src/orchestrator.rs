//! AiOrchestratorService — управление жизненным циклом обхода DPI (State Machine).

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::AiError;
use crate::fingerprint::NetworkFingerprint;
use crate::genome::StrategyGenome;

/// Состояния оркестратора.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestratorState {
    Idle,
    Fingerprinting,
    Selecting,
    Executing,
    Verifying,
    Evolving,
}

/// Конфигурация оркестратора.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub max_failures_before_evolve: u32,
    pub auto_evolve: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_failures_before_evolve: 3,
            auto_evolve: true,
        }
    }
}

/// Результат верификации (успех, неудача, latency).
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub success: bool,
    pub score: i32,
    pub latency_ms: f64,
}

/// Оркестратор — конечный автомат для выбора, запуска и эволюции DPI-стратегий.

/// События оркестратора.
#[derive(Debug, Clone)]
pub enum OrchestratorEvent {
    BeginFingerprinting,
    CompleteFingerprinting(NetworkFingerprint),
    BeginSelection,
    CompleteSelection(StrategyGenome),
    BeginExecution,
    CompleteExecution,
    CompleteVerification(VerificationResult),
    CompleteEvolution,
    Reset,
}

pub struct AiOrchestratorService {
    config: OrchestratorConfig,
    state: Arc<Mutex<OrchestratorState>>,
    current_fingerprint: Arc<Mutex<Option<NetworkFingerprint>>>,
    current_strategy: Arc<Mutex<Option<StrategyGenome>>>,
    failure_count: Arc<Mutex<u32>>,
}

impl AiOrchestratorService {
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(OrchestratorState::Idle)),
            current_fingerprint: Arc::new(Mutex::new(None)),
            current_strategy: Arc::new(Mutex::new(None)),
            failure_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Получить текущее состояние.
    pub async fn state(&self) -> OrchestratorState {
        *self.state.lock().await
    }

    /// Обработка события оркестратора (FSM).
    pub async fn dispatch(&self, event: OrchestratorEvent) -> Result<(), AiError> {
        let mut state = self.state.lock().await;

        match event {
            OrchestratorEvent::BeginFingerprinting => {
                if *state != OrchestratorState::Idle
                    && *state != OrchestratorState::Verifying
                    && *state != OrchestratorState::Evolving
                {
                    return Err(AiError::Orchestrator(format!(
                        "cannot start fingerprinting from state {:?}",
                        *state
                    )));
                }
                *state = OrchestratorState::Fingerprinting;
            }
            OrchestratorEvent::CompleteFingerprinting(fingerprint) => {
                if *state != OrchestratorState::Fingerprinting {
                    return Err(AiError::Orchestrator(format!(
                        "cannot complete fingerprinting from state {:?}",
                        *state
                    )));
                }
                *self.current_fingerprint.lock().await = Some(fingerprint);
                *state = OrchestratorState::Selecting;
            }
            OrchestratorEvent::BeginSelection => {
                if *state != OrchestratorState::Fingerprinting
                    && *state != OrchestratorState::Idle
                    && *state != OrchestratorState::Verifying
                    && *state != OrchestratorState::Evolving
                {
                    return Err(AiError::Orchestrator(format!(
                        "cannot start selection from state {:?}",
                        *state
                    )));
                }
                *state = OrchestratorState::Selecting;
            }
            OrchestratorEvent::CompleteSelection(strategy) => {
                if *state != OrchestratorState::Selecting {
                    return Err(AiError::Orchestrator(format!(
                        "cannot complete selection from state {:?}",
                        *state
                    )));
                }
                *self.current_strategy.lock().await = Some(strategy);
                *state = OrchestratorState::Executing;
            }
            OrchestratorEvent::BeginExecution => {
                if *state != OrchestratorState::Selecting && *state != OrchestratorState::Idle {
                    return Err(AiError::Orchestrator(format!(
                        "cannot start execution from state {:?}",
                        *state
                    )));
                }
                *state = OrchestratorState::Executing;
            }
            OrchestratorEvent::CompleteExecution => {
                if *state != OrchestratorState::Executing {
                    return Err(AiError::Orchestrator(format!(
                        "cannot complete execution from state {:?}",
                        *state
                    )));
                }
                *state = OrchestratorState::Verifying;
            }
            OrchestratorEvent::CompleteVerification(result) => {
                if *state != OrchestratorState::Verifying {
                    return Err(AiError::Orchestrator(format!(
                        "cannot complete verification from state {:?}",
                        *state
                    )));
                }

                if result.success {
                    *self.failure_count.lock().await = 0;
                    *state = OrchestratorState::Idle;
                } else {
                    let mut failures = self.failure_count.lock().await;
                    *failures += 1;

                    if self.config.auto_evolve && *failures >= self.config.max_failures_before_evolve {
                        *state = OrchestratorState::Evolving;
                        *failures = 0;
                    } else {
                        *state = OrchestratorState::Selecting;
                    }
                }
            }
            OrchestratorEvent::CompleteEvolution => {
                if *state != OrchestratorState::Evolving {
                    return Err(AiError::Orchestrator(format!(
                        "cannot complete evolution from state {:?}",
                        *state
                    )));
                }
                *state = OrchestratorState::Selecting;
            }
            OrchestratorEvent::Reset => {
                *state = OrchestratorState::Idle;
                *self.failure_count.lock().await = 0;
            }
        }
        Ok(())
    }

    pub async fn current_fingerprint(&self) -> Option<NetworkFingerprint> {
        self.current_fingerprint.lock().await.clone()
    }

    pub async fn current_strategy(&self) -> Option<StrategyGenome> {
        self.current_strategy.lock().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::{DpiEngineType, StrategyOrigin};

    #[tokio::test]
    async fn test_orchestrator_initial_state() {
        let orchestrator = AiOrchestratorService::new(OrchestratorConfig::default());
        assert_eq!(orchestrator.state().await, OrchestratorState::Idle);
    }

    #[tokio::test]
    async fn test_orchestrator_state_transitions() {
        let orchestrator = AiOrchestratorService::new(OrchestratorConfig::default());

        // Idle -> Fingerprinting
        orchestrator.dispatch(OrchestratorEvent::BeginFingerprinting).await.unwrap();
        assert_eq!(
            orchestrator.state().await,
            OrchestratorState::Fingerprinting
        );

        // Fingerprinting -> Selecting
        let fingerprint = NetworkFingerprint {
            hash: "testhash".to_string(),
            label: "test".to_string(),
            transport: "test".to_string(),
            gateway_ip: "1.2.3.4".to_string(),
            dns_servers: vec![],
            local_subnet: "1.2.3.0/24".to_string(),
            captured_at: chrono::Utc::now(),
        };
        orchestrator.dispatch(OrchestratorEvent::CompleteFingerprinting(fingerprint)).await
            .unwrap();
        assert_eq!(orchestrator.state().await, OrchestratorState::Selecting);

        // Selecting -> Executing
        let strategy = StrategyGenome::new(DpiEngineType::Zapret, StrategyOrigin::Builtin);
        orchestrator.dispatch(OrchestratorEvent::CompleteSelection(strategy)).await.unwrap();
        assert_eq!(orchestrator.state().await, OrchestratorState::Executing);

        // Executing -> Verifying
        orchestrator.dispatch(OrchestratorEvent::CompleteExecution).await.unwrap();
        assert_eq!(orchestrator.state().await, OrchestratorState::Verifying);

        // Verifying -> Idle (Success)
        let result = VerificationResult {
            success: true,
            score: 100,
            latency_ms: 50.0,
        };
        orchestrator.dispatch(OrchestratorEvent::CompleteVerification(result)).await.unwrap();
        assert_eq!(orchestrator.state().await, OrchestratorState::Idle);
    }

    #[tokio::test]
    async fn test_orchestrator_failure_triggers_evolution() {
        let config = OrchestratorConfig {
            max_failures_before_evolve: 2,
            auto_evolve: true,
        };
        let orchestrator = AiOrchestratorService::new(config);

        // Setup to Verifying state
        orchestrator.dispatch(OrchestratorEvent::BeginSelection).await.unwrap();
        let strategy = StrategyGenome::new(DpiEngineType::Zapret, StrategyOrigin::Builtin);
        orchestrator.dispatch(OrchestratorEvent::CompleteSelection(strategy)).await.unwrap();
        orchestrator.dispatch(OrchestratorEvent::CompleteExecution).await.unwrap();

        let fail_result = VerificationResult {
            success: false,
            score: 0,
            latency_ms: 1000.0,
        };

        // First failure -> Selecting
        orchestrator.dispatch(OrchestratorEvent::CompleteVerification(fail_result.clone())).await
            .unwrap();
        assert_eq!(orchestrator.state().await, OrchestratorState::Selecting);

        // Setup to Verifying state again
        let strategy2 = StrategyGenome::new(DpiEngineType::ByeDpi, StrategyOrigin::Builtin);
        orchestrator.dispatch(OrchestratorEvent::CompleteSelection(strategy2)).await.unwrap();
        orchestrator.dispatch(OrchestratorEvent::CompleteExecution).await.unwrap();

        // Second failure -> Evolving (because max_failures_before_evolve is 2)
        orchestrator.dispatch(OrchestratorEvent::CompleteVerification(fail_result)).await
            .unwrap();
        assert_eq!(orchestrator.state().await, OrchestratorState::Evolving);

        // Evolving -> Selecting
        orchestrator.dispatch(OrchestratorEvent::CompleteEvolution).await.unwrap();
        assert_eq!(orchestrator.state().await, OrchestratorState::Selecting);
    }
}
