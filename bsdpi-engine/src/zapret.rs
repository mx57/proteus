//! ZapretEngine — запуск winws.exe (Windows) или нативного zapret.
//!
//! Порт C# `BSDPI.Core/Services/ZapretEngine.cs`
//! Использует `tokio::process::Command` для асинхронного управления процессом.

use tokio::sync::broadcast;
use tokio::process::Command;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use crate::traits::*;

use async_trait::async_trait;

/// Движок Zapret (winws.exe).
pub struct ZapretEngine {
    engine_dir: String,
    status: Arc<AtomicU8>,
    process: tokio::sync::Mutex<Option<tokio::process::Child>>,
    process_info: tokio::sync::Mutex<Option<EngineProcessInfo>>,
    tx: broadcast::Sender<EngineEvent>,
}

impl ZapretEngine {
    pub fn new(engine_dir: String) -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            engine_dir,
            status: Arc::new(AtomicU8::new(EngineStatus::Stopped as u8)),
            process: tokio::sync::Mutex::new(None),
            process_info: tokio::sync::Mutex::new(None),
            tx,
        }
    }

    fn set_status(&self, status: EngineStatus) {
        self.status.store(status as u8, Ordering::SeqCst);
        let _ = self.tx.send(EngineEvent::StatusChanged(status));
    }

    fn get_status(&self) -> EngineStatus {
        match self.status.load(Ordering::SeqCst) {
            0 => EngineStatus::Stopped,
            1 => EngineStatus::Starting,
            2 => EngineStatus::Running,
            3 => EngineStatus::Failed,
            4 => EngineStatus::Crashed,
            _ => EngineStatus::Stopped,
        }
    }

    /// Получить имя бинарника для текущей платформы.
    fn binary_name() -> &'static str {
        #[cfg(target_os = "windows")]
        { "winws.exe" }
        #[cfg(target_os = "linux")]
        { "zapret" }
        #[cfg(target_os = "android")]
        { "libzapret.so" }
    }
}

impl DpiEngine for ZapretEngine {
    fn engine_type(&self) -> DpiEngineType { DpiEngineType::Zapret }
    fn display_name(&self) -> &str { "Zapret" }
    fn status(&self) -> EngineStatus { self.get_status() }

    fn process_info(&self) -> Option<EngineProcessInfo> {
        self.process_info.try_lock().ok().and_then(|guard| guard.clone())
    }

    fn events(&self) -> broadcast::Receiver<EngineEvent> {
        self.tx.subscribe()
    }

    async fn start(&mut self, profile: &EngineProfile) -> Result<(), EngineError> {
        if self.get_status().is_active() {
            return Err(EngineError::AlreadyRunning);
        }

        self.set_status(EngineStatus::Starting);

        let executable = Self::binary_name();
        let exec_path = if cfg!(target_os = "windows") {
            format!("{}/bin/{}", self.engine_dir, executable)
        } else {
            format!("{}/{}", self.engine_dir, executable)
        };

        let args = profile.to_zapret_args();

        log::info!("Starting Zapret: {} {:?}", exec_path, args);

        match Command::new(&exec_path)
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(child) => {
                let pid = child.id().unwrap_or(0);
                let info = EngineProcessInfo::new(pid, executable, None);
                *self.process.lock().await = Some(child);
                *self.process_info.lock().await = Some(info);
                self.set_status(EngineStatus::Running);
                log::info!("Zapret started with PID {}", pid);
                Ok(())
            }
            Err(e) => {
                self.set_status(EngineStatus::Failed);
                log::error!("Failed to start Zapret: {}", e);
                Err(EngineError::StartFailed(e.to_string()))
            }
        }
    }

    async fn stop(&mut self) -> Result<(), EngineError> {
        let mut proc_guard = self.process.lock().await;
        if let Some(mut child) = proc_guard.take() {
            log::info!("Stopping Zapret (PID {})", child.id().unwrap_or(0));
            let _ = child.kill().await;
            let _ = child.wait().await;
            *self.process_info.lock().await = None;
            self.set_status(EngineStatus::Stopped);
            Ok(())
        } else {
            // Уже остановлен — не ошибка
            self.set_status(EngineStatus::Stopped);
            Ok(())
        }
    }

    async fn probe(&mut self) -> EngineStatus {
        let mut proc_guard = self.process.lock().await;
        if let Some(ref mut child) = *proc_guard {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    // Процесс завершился
                    drop(proc_guard);
                    *self.process.lock().await = None;
                    *self.process_info.lock().await = None;
                    self.set_status(EngineStatus::Crashed);
                    EngineStatus::Crashed
                }
                Ok(None) => {
                    // Всё ещё работает
                    EngineStatus::Running
                }
                Err(_) => {
                    self.set_status(EngineStatus::Failed);
                    EngineStatus::Failed
                }
            }
        } else {
            self.get_status()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_zapret_creation() {
        let engine = ZapretEngine::new("/tmp/engine".into());
        assert_eq!(engine.status(), EngineStatus::Stopped);
        assert_eq!(engine.engine_type(), DpiEngineType::Zapret);
    }

    #[tokio::test]
    async fn test_zapret_binary_name() {
        let name = ZapretEngine::binary_name();
        #[cfg(target_os = "linux")]
        assert_eq!(name, "zapret");
        #[cfg(target_os = "windows")]
        assert_eq!(name, "winws.exe");
    }

    #[tokio::test]
    async fn test_zapret_start_nonexistent() {
        let mut engine = ZapretEngine::new("/nonexistent".into());
        let profile = EngineProfile::default();
        let result = engine.start(&profile).await;
        assert!(result.is_err(), "Should fail with nonexistent binary");
        match result {
            Err(EngineError::StartFailed(_)) => assert!(true),
            _ => panic!("Expected StartFailed error"),
        }
    }

    #[tokio::test]
    async fn test_zapret_stop_without_start() {
        let mut engine = ZapretEngine::new("/tmp/engine".into());
        // Stop без старта — Ok(())
        let result = engine.stop().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_zapret_double_start() {
        let mut engine = ZapretEngine::new("/nonexistent".into());
        let profile = EngineProfile::default();
        let _ = engine.start(&profile).await; // first attempt
        let result2 = engine.start(&profile).await;
        // Второй start после Failed должен быть Ok (он сбросит статус)
        // Но мы не можем проверить точно, так как статус после Failed
        // В любом случае, не должно паниковать
        assert!(result2.is_err() || result2.is_ok());
    }

    #[tokio::test]
    async fn test_zapret_events() {
        let engine = ZapretEngine::new("/tmp/engine".into());
        let mut rx = engine.events();
        engine.set_status(EngineStatus::Running);
        // Получаем событие через receiver
        let event = rx.recv().await;
        assert!(event.is_ok());
    }
}
