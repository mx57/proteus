//! OrchestratorService — state machine for DPI bypass lifecycle.

use crate::error::AiError;
use serde::{Deserialize, Serialize};

/// State of the AI Orchestrator finite state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrchestratorState {
    Idle,
    Fingerprinting,
    Selecting,
    Executing,
    Verifying,
    Evolving,
}

/// Orchestrator service that manages the lifecycle of DPI bypass strategies.
pub struct AiOrchestratorService {
    state: OrchestratorState,
}

impl AiOrchestratorService {
    /// Creates a new orchestrator in the Idle state.
    pub fn new() -> Self {
        Self {
            state: OrchestratorState::Idle,
        }
    }

    /// Returns the current state.
    pub fn state(&self) -> OrchestratorState {
        self.state
    }

    /// Transitions to the Fingerprinting state.
    pub fn start_fingerprinting(&mut self) -> Result<(), AiError> {
        if self.state == OrchestratorState::Idle {
            self.state = OrchestratorState::Fingerprinting;
            Ok(())
        } else {
            Err(AiError::Orchestrator(format!(
                "invalid transition: {:?} -> Fingerprinting",
                self.state
            )))
        }
    }

    /// Transitions to the Selecting state.
    pub fn start_selecting(&mut self) -> Result<(), AiError> {
        if self.state == OrchestratorState::Fingerprinting {
            self.state = OrchestratorState::Selecting;
            Ok(())
        } else {
            Err(AiError::Orchestrator(format!(
                "invalid transition: {:?} -> Selecting",
                self.state
            )))
        }
    }

    /// Transitions to the Executing state.
    pub fn start_executing(&mut self) -> Result<(), AiError> {
        if self.state == OrchestratorState::Selecting {
            self.state = OrchestratorState::Executing;
            Ok(())
        } else {
            Err(AiError::Orchestrator(format!(
                "invalid transition: {:?} -> Executing",
                self.state
            )))
        }
    }

    /// Transitions to the Verifying state.
    pub fn start_verifying(&mut self) -> Result<(), AiError> {
        if self.state == OrchestratorState::Executing {
            self.state = OrchestratorState::Verifying;
            Ok(())
        } else {
            Err(AiError::Orchestrator(format!(
                "invalid transition: {:?} -> Verifying",
                self.state
            )))
        }
    }

    /// Transitions to the Evolving state.
    pub fn start_evolving(&mut self) -> Result<(), AiError> {
        if self.state == OrchestratorState::Verifying {
            self.state = OrchestratorState::Evolving;
            Ok(())
        } else {
            Err(AiError::Orchestrator(format!(
                "invalid transition: {:?} -> Evolving",
                self.state
            )))
        }
    }

    /// Resets the orchestrator back to Idle.
    pub fn reset_to_idle(&mut self) {
        self.state = OrchestratorState::Idle;
    }
}

impl Default for AiOrchestratorService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        let mut orchestrator = AiOrchestratorService::new();
        assert_eq!(orchestrator.state(), OrchestratorState::Idle);

        assert!(orchestrator.start_fingerprinting().is_ok());
        assert_eq!(orchestrator.state(), OrchestratorState::Fingerprinting);

        assert!(orchestrator.start_selecting().is_ok());
        assert_eq!(orchestrator.state(), OrchestratorState::Selecting);

        assert!(orchestrator.start_executing().is_ok());
        assert_eq!(orchestrator.state(), OrchestratorState::Executing);

        assert!(orchestrator.start_verifying().is_ok());
        assert_eq!(orchestrator.state(), OrchestratorState::Verifying);

        assert!(orchestrator.start_evolving().is_ok());
        assert_eq!(orchestrator.state(), OrchestratorState::Evolving);

        orchestrator.reset_to_idle();
        assert_eq!(orchestrator.state(), OrchestratorState::Idle);
    }

    #[test]
    fn test_invalid_transitions() {
        let mut orchestrator = AiOrchestratorService::new();
        assert_eq!(orchestrator.state(), OrchestratorState::Idle);

        // Cannot jump to Executing from Idle
        let err = orchestrator.start_executing();
        assert!(err.is_err());
        assert_eq!(orchestrator.state(), OrchestratorState::Idle);

        // Valid transition
        assert!(orchestrator.start_fingerprinting().is_ok());

        // Cannot jump to Verifying from Fingerprinting
        let err = orchestrator.start_verifying();
        assert!(err.is_err());
        assert_eq!(orchestrator.state(), OrchestratorState::Fingerprinting);
    }
}
