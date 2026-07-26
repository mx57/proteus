use crate::error::AiError;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Состояния конечного автомата, некоторые с ассоциированным контекстом.
#[derive(Debug, Clone, PartialEq)]
pub enum OrchestratorState {
    Idle,
    Fingerprinting,
    Selecting { network_hash: String },
    Executing { network_hash: String, strategy_id: Uuid },
    Verifying { network_hash: String, strategy_id: Uuid },
    Evolving { network_hash: String },
}

/// События, вызывающие переходы между состояниями.
#[derive(Debug, Clone)]
pub enum OrchestratorEvent {
    Start,
    FingerprintAcquired { network_hash: String },
    SelectionMade { strategy_id: Uuid },
    ExecutionStarted,
    VerificationCompleted,
    EvolutionCompleted,
    Fault { reason: String },
    Reset,
}

/// Конфигурация оркестратора.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub max_retries: usize,
    pub timeout_ms: u64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            timeout_ms: 5000,
        }
    }
}

/// Сервис оркестрации AI (конечный автомат).
pub struct AiOrchestratorService {
    _config: OrchestratorConfig,
    state: Arc<Mutex<OrchestratorState>>,
}

impl AiOrchestratorService {
    /// Создать новый оркестратор.
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            _config: config,
            state: Arc::new(Mutex::new(OrchestratorState::Idle)),
        }
    }

    /// Получить текущее состояние.
    pub async fn get_state(&self) -> OrchestratorState {
        self.state.lock().await.clone()
    }

    /// Обработка события и переход в новое состояние.
    pub async fn process_event(&self, event: OrchestratorEvent) -> Result<OrchestratorState, AiError> {
        let mut state = self.state.lock().await;

        let next_state = match (&*state, event) {
            // Обработка Fault из любого состояния, возвращаемся в Idle
            (_, OrchestratorEvent::Fault { reason }) => {
                log::warn!("Orchestrator fault: {reason}");
                OrchestratorState::Idle
            }
            // Обработка Reset из любого состояния
            (_, OrchestratorEvent::Reset) => OrchestratorState::Idle,

            // Нормальные переходы
            (OrchestratorState::Idle, OrchestratorEvent::Start) => OrchestratorState::Fingerprinting,

            (OrchestratorState::Fingerprinting, OrchestratorEvent::FingerprintAcquired { network_hash }) => {
                OrchestratorState::Selecting { network_hash }
            }

            (OrchestratorState::Selecting { network_hash }, OrchestratorEvent::SelectionMade { strategy_id }) => {
                OrchestratorState::Executing {
                    network_hash: network_hash.clone(),
                    strategy_id,
                }
            }

            (OrchestratorState::Executing { network_hash, strategy_id }, OrchestratorEvent::ExecutionStarted) => {
                OrchestratorState::Verifying {
                    network_hash: network_hash.clone(),
                    strategy_id: *strategy_id,
                }
            }

            (OrchestratorState::Verifying { network_hash, .. }, OrchestratorEvent::VerificationCompleted) => {
                OrchestratorState::Evolving {
                    network_hash: network_hash.clone(),
                }
            }

            (OrchestratorState::Evolving { .. }, OrchestratorEvent::EvolutionCompleted) => {
                OrchestratorState::Idle
            }

            // Недопустимые переходы
            (current_state, event) => {
                return Err(AiError::Orchestrator(format!(
                    "Invalid state transition from {:?} via event {:?}",
                    current_state, event
                )));
            }
        };

        *state = next_state.clone();
        Ok(next_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_initial_state() {
        let orchestrator = AiOrchestratorService::new(OrchestratorConfig::default());
        assert_eq!(orchestrator.get_state().await, OrchestratorState::Idle);
    }

    #[tokio::test]
    async fn test_orchestrator_happy_path() {
        let orchestrator = AiOrchestratorService::new(OrchestratorConfig::default());
        let net_hash = "test-hash".to_string();
        let sid = Uuid::new_v4();

        assert_eq!(
            orchestrator.process_event(OrchestratorEvent::Start).await.unwrap(),
            OrchestratorState::Fingerprinting
        );

        assert_eq!(
            orchestrator.process_event(OrchestratorEvent::FingerprintAcquired { network_hash: net_hash.clone() }).await.unwrap(),
            OrchestratorState::Selecting { network_hash: net_hash.clone() }
        );

        assert_eq!(
            orchestrator.process_event(OrchestratorEvent::SelectionMade { strategy_id: sid }).await.unwrap(),
            OrchestratorState::Executing { network_hash: net_hash.clone(), strategy_id: sid }
        );

        assert_eq!(
            orchestrator.process_event(OrchestratorEvent::ExecutionStarted).await.unwrap(),
            OrchestratorState::Verifying { network_hash: net_hash.clone(), strategy_id: sid }
        );

        assert_eq!(
            orchestrator.process_event(OrchestratorEvent::VerificationCompleted).await.unwrap(),
            OrchestratorState::Evolving { network_hash: net_hash }
        );

        assert_eq!(
            orchestrator.process_event(OrchestratorEvent::EvolutionCompleted).await.unwrap(),
            OrchestratorState::Idle
        );
    }

    #[tokio::test]
    async fn test_orchestrator_invalid_transition() {
        let orchestrator = AiOrchestratorService::new(OrchestratorConfig::default());
        let res = orchestrator.process_event(OrchestratorEvent::ExecutionStarted).await;
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), AiError::Orchestrator(_)));
    }

    #[tokio::test]
    async fn test_orchestrator_fault_recovery() {
        let orchestrator = AiOrchestratorService::new(OrchestratorConfig::default());
        orchestrator.process_event(OrchestratorEvent::Start).await.unwrap();
        assert_eq!(orchestrator.get_state().await, OrchestratorState::Fingerprinting);

        orchestrator.process_event(OrchestratorEvent::Fault { reason: "err".into() }).await.unwrap();
        assert_eq!(orchestrator.get_state().await, OrchestratorState::Idle);
    }
}
