//! # BSDPI Core Services
//!
//! Core-сервисы DPI-обхода: проверка доступности, управление движками, настройки, обновления.

pub mod probe;
pub mod manager;
pub mod settings;
pub mod chains;
pub mod updater;

pub use probe::{ProbeService, ProbeResult, TargetEntry, ProbeOptions, CheckResult};
pub use manager::{DpiEngineManager, ManagerConfig};
pub use settings::{SettingsService, AppSettings};
pub use chains::{ChainBuilder, ChainMode, ChainLink};
pub use updater::{SelfUpdater, UpdateInfo, UpdateChannel};
