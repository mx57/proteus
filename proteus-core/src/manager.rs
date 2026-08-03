//! DpiEngineManager — управление группой DPI-движков.
//!
//! Порт C# `Proteus.Core/Services/DpiEngineManager.cs`

use crate::probe::{ProbeOptions, ProbeResult, ProbeService, TargetEntry};
use proteus_engine::{ByeDpiEngine, DpiEngine, EngineProfile, EngineStatus as EngStatus, WarpEngine, ZapretEngine};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Тип DPI-движка (локальное определение, без зависимости от proteus-engine).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ManagedEngineType {
    Zapret,
    ByeDpi,
    Warp,
    Hybrid,
    Chained,
    None,
}

impl ManagedEngineType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ManagedEngineType::Zapret => "zapret",
            ManagedEngineType::ByeDpi => "byedpi",
            ManagedEngineType::Warp => "warp",
            ManagedEngineType::Hybrid => "hybrid",
            ManagedEngineType::Chained => "chained",
            ManagedEngineType::None => "none",
        }
    }
}

/// Статус управляемого движка.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineStatus {
    Stopped,
    Starting,
    Running,
    Failed,
    Crashed,
}

impl From<EngStatus> for EngineStatus {
    fn from(status: EngStatus) -> Self {
        match status {
            EngStatus::Stopped => EngineStatus::Stopped,
            EngStatus::Starting => EngineStatus::Starting,
            EngStatus::Running => EngineStatus::Running,
            EngStatus::Failed => EngineStatus::Failed,
            EngStatus::Crashed => EngineStatus::Crashed,
        }
    }
}

/// Конфигурация менеджера.
#[derive(Debug, Clone)]
pub struct ManagerConfig {
    pub auto_restart: bool,
    pub max_restart_attempts: u32,
    pub probe_on_start: bool,
    pub engine_dir: String,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            auto_restart: true,
            max_restart_attempts: 3,
            probe_on_start: true,
            engine_dir: "engine".into(),
        }
    }
}

/// Запись о состоянии движка.
struct EngineEntry {
    engine_type: ManagedEngineType,
    status: EngineStatus,
    restart_count: u32,
    display_name: String,
}

/// DpiEngineManager — управление несколькими DPI-движками.
pub struct DpiEngineManager {
    pub config: ManagerConfig,
    engines_info: Arc<Mutex<Vec<EngineEntry>>>,
    active_profile: Arc<Mutex<Option<ManagedEngineType>>>,
    probe_service: ProbeService,
    active_engine_instances: Arc<Mutex<HashMap<ManagedEngineType, Box<dyn DpiEngine>>>>,
}

impl DpiEngineManager {
    pub fn new(config: ManagerConfig) -> Self {
        Self {
            config,
            engines_info: Arc::new(Mutex::new(Vec::new())),
            active_profile: Arc::new(Mutex::new(None)),
            probe_service: ProbeService::new(),
            active_engine_instances: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Зарегистрировать движок.
    pub async fn register(&self, engine_type: ManagedEngineType, display_name: &str) {
        let mut engines = self.engines_info.lock().await;
        if !engines.iter().any(|e| e.engine_type == engine_type) {
            engines.push(EngineEntry {
                engine_type,
                status: EngineStatus::Stopped,
                restart_count: 0,
                display_name: display_name.into(),
            });
            log::info!(
                "Registered engine: {} ({})",
                display_name,
                engine_type.as_str()
            );
        }
    }

    /// Запустить движок.
    pub async fn start(&self, engine_type: ManagedEngineType) -> Result<(), String> {
        let mut engines = self.engines_info.lock().await;
        let engine_entry = engines.iter_mut().find(|e| e.engine_type == engine_type);

        if let Some(engine) = engine_entry {
            if engine.status == EngineStatus::Running {
                return Err("already running".into());
            }
            engine.status = EngineStatus::Starting;
            engine.restart_count = 0;

            let display_name = engine.display_name.clone();

            let mut active_instances = self.active_engine_instances.lock().await;

            let mut dpi_engine: Box<dyn DpiEngine> = match engine_type {
                ManagedEngineType::Zapret => Box::new(ZapretEngine::new(self.config.engine_dir.clone())),
                ManagedEngineType::ByeDpi => Box::new(ByeDpiEngine::new(self.config.engine_dir.clone())),
                ManagedEngineType::Warp => Box::new(WarpEngine::new(self.config.engine_dir.clone())),
                _ => {
                    engine.status = EngineStatus::Failed;
                    return Err(format!("Engine type {:?} is not directly startable", engine_type));
                }
            };

            // In a real scenario, the profile should be passed here or fetched from elsewhere
            let profile = EngineProfile::default();

            // Handle testing environment by mocking the start if not really starting Zapret binary
            #[cfg(test)]
            std::env::set_var("PROTEUS_MOCK_ENGINE", "1");

            match dpi_engine.start(&profile).await {
                Ok(_) => {
                    engine.status = EngineStatus::Running;
                    *self.active_profile.lock().await = Some(engine_type);
                    active_instances.insert(engine_type, dpi_engine);
                    log::info!("Started engine: {}", display_name);
                    Ok(())
                }
                Err(e) => {
                    engine.status = EngineStatus::Failed;
                    log::error!("Failed to start engine {}: {:?}", display_name, e);
                    Err(format!("Failed to start: {:?}", e))
                }
            }

        } else {
            Err(format!("engine {:?} not registered", engine_type))
        }
    }

    /// Остановить движок.
    pub async fn stop(&self, engine_type: ManagedEngineType) -> Result<(), String> {
        let mut engines = self.engines_info.lock().await;
        if let Some(engine) = engines.iter_mut().find(|e| e.engine_type == engine_type) {

            let mut active_instances = self.active_engine_instances.lock().await;

            if let Some(mut dpi_engine) = active_instances.remove(&engine_type) {
                if let Err(e) = dpi_engine.stop().await {
                     log::error!("Error stopping engine {}: {:?}", engine.display_name, e);
                }
            }

            engine.status = EngineStatus::Stopped;
            log::info!("Stopped engine: {}", engine.display_name);
            if *self.active_profile.lock().await == Some(engine_type) {
                *self.active_profile.lock().await = None;
            }
            Ok(())
        } else {
            Err(format!("engine {:?} not registered", engine_type))
        }
    }

    /// Получить статус движка.
    pub async fn status(&self, engine_type: ManagedEngineType) -> Option<EngineStatus> {
        let mut engines = self.engines_info.lock().await;
        let engine_entry = engines.iter_mut().find(|e| e.engine_type == engine_type);

        if let Some(engine) = engine_entry {
             let mut active_instances = self.active_engine_instances.lock().await;
             if let Some(dpi_engine) = active_instances.get_mut(&engine_type) {
                  let current_status = dpi_engine.probe().await;
                  engine.status = current_status.into();
             }
             Some(engine.status)
        } else {
            None
        }
    }

    /// Получить активный движок.
    pub async fn active_engine(&self) -> Option<ManagedEngineType> {
        *self.active_profile.lock().await
    }

    /// Проверить доступность через активный движок.
    pub async fn probe_active(&self, targets: &[TargetEntry]) -> ProbeResult {
        let opts = ProbeOptions::default();
        self.probe_service.check_all(targets, &opts).await
    }

    /// Список зарегистрированных движков.
    pub async fn list_engines(&self) -> Vec<(ManagedEngineType, EngineStatus, String)> {
        let engines = self.engines_info.lock().await;
        engines
            .iter()
            .map(|e| (e.engine_type, e.status, e.display_name.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let mgr = DpiEngineManager::new(ManagerConfig::default());
        let list = mgr.list_engines().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_register_and_start() {
        let mgr = DpiEngineManager::new(ManagerConfig::default());
        mgr.register(ManagedEngineType::Zapret, "Zapret").await;
        mgr.start(ManagedEngineType::Zapret).await.unwrap();
        assert_eq!(
            mgr.status(ManagedEngineType::Zapret).await,
            Some(EngineStatus::Running)
        );
    }

    #[tokio::test]
    async fn test_double_start_fails() {
        let mgr = DpiEngineManager::new(ManagerConfig::default());
        mgr.register(ManagedEngineType::Zapret, "Zapret").await;
        mgr.start(ManagedEngineType::Zapret).await.unwrap();
        let result = mgr.start(ManagedEngineType::Zapret).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stop() {
        let mgr = DpiEngineManager::new(ManagerConfig::default());
        mgr.register(ManagedEngineType::ByeDpi, "ByeDPI").await;
        mgr.start(ManagedEngineType::ByeDpi).await.unwrap();
        mgr.stop(ManagedEngineType::ByeDpi).await.unwrap();
        assert_eq!(
            mgr.status(ManagedEngineType::ByeDpi).await,
            Some(EngineStatus::Stopped)
        );
    }

    #[tokio::test]
    async fn test_active_engine() {
        let mgr = DpiEngineManager::new(ManagerConfig::default());
        assert!(mgr.active_engine().await.is_none());
        mgr.register(ManagedEngineType::Zapret, "Zapret").await;
        mgr.start(ManagedEngineType::Zapret).await.unwrap();
        assert_eq!(mgr.active_engine().await, Some(ManagedEngineType::Zapret));
    }

    #[test]
    fn test_engine_type_as_str() {
        assert_eq!(ManagedEngineType::Zapret.as_str(), "zapret");
        assert_eq!(ManagedEngineType::Hybrid.as_str(), "hybrid");
    }
}
