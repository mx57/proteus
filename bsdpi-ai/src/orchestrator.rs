//! AiOrchestratorService — State machine for orchestrating DPI bypass AI logic.
//!
//! Coordinates fingerprinting, strategy selection, execution, and evolution.
//!
//! ## C# оригинал
//! `BSDPI.AI/Services/AiOrchestratorService.cs`

use crate::error::AiError;
use crate::genome::StrategyGenome;
use crate::fingerprint::NetworkFingerprint;

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

impl Default for OrchestratorState {
    fn default() -> Self {
        Self::Idle
    }
}

/// Конфигурация оркестратора.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub max_retries: u32,
    pub auto_evolve: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            auto_evolve: true,
        }
    }
}

/// Оркестратор AI. Координирует процесс обхода DPI.
pub struct AiOrchestratorService {
    state: OrchestratorState,
    config: OrchestratorConfig,
    current_fingerprint: Option<NetworkFingerprint>,
    current_strategy: Option<StrategyGenome>,
}

impl AiOrchestratorService {
    /// Создает новый оркестратор.
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            state: OrchestratorState::Idle,
            config,
            current_fingerprint: None,
            current_strategy: None,
        }
    }

    /// Текущее состояние.
    pub fn state(&self) -> OrchestratorState {
        self.state
    }

    /// Начать процесс fingerprinting.
    pub fn start_fingerprinting(&mut self) -> Result<(), AiError> {
        if self.state != OrchestratorState::Idle && self.state != OrchestratorState::Verifying {
            return Err(AiError::Orchestrator(format!(
                "Cannot start fingerprinting from state {:?}",
                self.state
            )));
        }
        self.state = OrchestratorState::Fingerprinting;
        Ok(())
    }

    /// Завершить fingerprinting.
    pub fn complete_fingerprinting(&mut self, fp: NetworkFingerprint) -> Result<(), AiError> {
        if self.state != OrchestratorState::Fingerprinting {
            return Err(AiError::Orchestrator(format!(
                "Cannot complete fingerprinting from state {:?}",
                self.state
            )));
        }
        self.current_fingerprint = Some(fp);
        self.state = OrchestratorState::Selecting;
        Ok(())
    }

    /// Получить текущий выбранный fingerprint сети.
    pub fn current_fingerprint(&self) -> Option<&NetworkFingerprint> {
        self.current_fingerprint.as_ref()
    }

    /// Получить текущую выбранную стратегию.
    pub fn current_strategy(&self) -> Option<&StrategyGenome> {
        self.current_strategy.as_ref()
    }

    /// Завершить выбор стратегии.
    pub fn complete_selection(&mut self, strategy: StrategyGenome) -> Result<(), AiError> {
        if self.state != OrchestratorState::Selecting {
            return Err(AiError::Orchestrator(format!(
                "Cannot complete selection from state {:?}",
                self.state
            )));
        }
        self.current_strategy = Some(strategy);
        // После выбора переходим в состояние выполнения (ожидаем запуска движка)
        self.state = OrchestratorState::Executing;
        Ok(())
    }

    /// Завершить выполнение и перейти к проверке.
    pub fn complete_execution(&mut self) -> Result<(), AiError> {
        if self.state != OrchestratorState::Executing {
            return Err(AiError::Orchestrator(format!(
                "Cannot complete execution from state {:?}",
                self.state
            )));
        }
        // В реальном приложении здесь будет логика перехода после запуска движка
        self.state = OrchestratorState::Verifying;
        Ok(())
    }

    /// Успешная проверка.
    pub fn verify_success(&mut self) -> Result<(), AiError> {
        if self.state != OrchestratorState::Verifying {
            return Err(AiError::Orchestrator(format!(
                "Cannot verify from state {:?}",
                self.state
            )));
        }
        self.state = OrchestratorState::Idle;
        Ok(())
    }

    /// Проверка не удалась, нужно перевыбрать или эволюционировать.
    pub fn verify_failure(&mut self, evolve: bool) -> Result<(), AiError> {
        if self.state != OrchestratorState::Verifying {
            return Err(AiError::Orchestrator(format!(
                "Cannot verify from state {:?}",
                self.state
            )));
        }
        if evolve && self.config.auto_evolve {
            self.state = OrchestratorState::Evolving;
        } else {
            self.state = OrchestratorState::Selecting;
        }
        Ok(())
    }

    /// Завершить эволюцию.
    pub fn complete_evolution(&mut self) -> Result<(), AiError> {
        if self.state != OrchestratorState::Evolving {
             return Err(AiError::Orchestrator(format!(
                "Cannot complete evolution from state {:?}",
                self.state
            )));
        }
        self.state = OrchestratorState::Selecting;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_idle() {
        let orchestrator = AiOrchestratorService::new(OrchestratorConfig::default());
        assert_eq!(orchestrator.state(), OrchestratorState::Idle);
    }

    #[test]
    fn test_orchestrator_config_default() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.auto_evolve, true);
    }

    #[test]
    fn test_state_transition_idle_to_fingerprinting() {
        let mut orchestrator = AiOrchestratorService::new(OrchestratorConfig::default());
        assert!(orchestrator.start_fingerprinting().is_ok());
        assert_eq!(orchestrator.state(), OrchestratorState::Fingerprinting);
    }

    #[test]
    fn test_invalid_state_transition() {
        let mut orchestrator = AiOrchestratorService::new(OrchestratorConfig::default());
        // Can't complete fingerprinting if not in Fingerprinting state
        let fp = NetworkFingerprint {
            hash: "test".to_string(),
            transport: "tcp".to_string(),
            gateway_ip: "1.2.3.4".to_string(),
            dns_servers: vec![],
            local_subnet: "1.2.3.0/24".to_string(),
            label: "test".to_string(),
            captured_at: chrono::Utc::now(),
        };
        assert!(orchestrator.complete_fingerprinting(fp).is_err());
    }

    #[test]
    fn test_full_successful_cycle() {
        let mut orchestrator = AiOrchestratorService::new(OrchestratorConfig::default());

        let fp = NetworkFingerprint {
            hash: "test".to_string(),
            transport: "tcp".to_string(),
            gateway_ip: "1.2.3.4".to_string(),
            dns_servers: vec![],
            local_subnet: "1.2.3.0/24".to_string(),
            label: "test".to_string(),
            captured_at: chrono::Utc::now(),
        };

        let strategy = StrategyGenome::default_zapret();

        assert!(orchestrator.start_fingerprinting().is_ok());
        assert!(orchestrator.complete_fingerprinting(fp).is_ok());

        assert!(orchestrator.current_fingerprint().is_some());

        assert!(orchestrator.complete_selection(strategy).is_ok());

        assert!(orchestrator.current_strategy().is_some());

        assert!(orchestrator.complete_execution().is_ok());
        assert!(orchestrator.verify_success().is_ok());
        assert_eq!(orchestrator.state(), OrchestratorState::Idle);
    }

    #[test]
    fn test_verification_failure_with_evolution() {
        let mut orchestrator = AiOrchestratorService::new(OrchestratorConfig::default());

        let fp = NetworkFingerprint {
            hash: "test".to_string(),
            transport: "tcp".to_string(),
            gateway_ip: "1.2.3.4".to_string(),
            dns_servers: vec![],
            local_subnet: "1.2.3.0/24".to_string(),
            label: "test".to_string(),
            captured_at: chrono::Utc::now(),
        };

        let strategy = StrategyGenome::default_zapret();

        assert!(orchestrator.start_fingerprinting().is_ok());
        assert!(orchestrator.complete_fingerprinting(fp).is_ok());
        assert!(orchestrator.complete_selection(strategy).is_ok());
        assert!(orchestrator.complete_execution().is_ok());
        assert!(orchestrator.verify_failure(true).is_ok());
        assert_eq!(orchestrator.state(), OrchestratorState::Evolving);
        assert!(orchestrator.complete_evolution().is_ok());
        assert_eq!(orchestrator.state(), OrchestratorState::Selecting);
    }
}
