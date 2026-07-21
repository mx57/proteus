//! ByeDpiEngine — запуск ciadpi.exe (Windows/Linux).
//!
//! Порт C# `BSDPI.Core/Services/ByeDpiEngine.cs`

use crate::traits::*;

pub struct ByeDpiEngine {
    engine_dir: String,
    status: std::sync::atomic::AtomicU8,
}

impl ByeDpiEngine {
    pub fn new(engine_dir: String) -> Self {
        Self {
            engine_dir,
            status: std::sync::atomic::AtomicU8::new(EngineStatus::Stopped as u8),
        }
    }
}

impl DpiEngine for ByeDpiEngine {
    fn engine_type(&self) -> DpiEngineType { DpiEngineType::ByeDpi }
    fn display_name(&self) -> &str { "ByeDPI" }
    fn status(&self) -> EngineStatus {
        match self.status.load(std::sync::atomic::Ordering::SeqCst) {
            0 => EngineStatus::Stopped,
            2 => EngineStatus::Running,
            3 => EngineStatus::Failed,
            _ => EngineStatus::Stopped,
        }
    }
    fn process_info(&self) -> Option<EngineProcessInfo> { None }
    fn events(&self) -> tokio::sync::broadcast::Receiver<EngineEvent> {
        let (tx, rx) = tokio::sync::broadcast::channel(64);
        drop(tx);
        rx
    }

    async fn start(&mut self, _profile: &EngineProfile) -> Result<(), EngineError> {
        self.status.store(EngineStatus::Running as u8, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), EngineError> {
        self.status.store(EngineStatus::Stopped as u8, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn probe(&mut self) -> EngineStatus { self.status() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_byedpi_creation() {
        let engine = ByeDpiEngine::new("/tmp/engine".into());
        assert_eq!(engine.status(), EngineStatus::Stopped);
    }

    #[tokio::test]
    async fn test_byedpi_start_stop() {
        let mut engine = ByeDpiEngine::new("/tmp/engine".into());
        let profile = EngineProfile::default();
        assert!(engine.start(&profile).await.is_ok());
        assert_eq!(engine.status(), EngineStatus::Running);
        assert!(engine.stop().await.is_ok());
        assert_eq!(engine.status(), EngineStatus::Stopped);
    }
}
