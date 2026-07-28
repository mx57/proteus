//! # Proteus Core Services
//!
//! Core-сервисы DPI-обхода: проверка доступности, управление движками, настройки, обновления.

pub mod chains;
pub mod manager;
pub mod probe;
pub mod settings;
pub mod updater;

pub use chains::{ChainBuilder, ChainLink, ChainMode};
pub use manager::{DpiEngineManager, ManagerConfig};
pub use probe::{CheckResult, ProbeOptions, ProbeResult, ProbeService, TargetEntry};
pub use settings::{AppSettings, SettingsService};
pub use updater::{SelfUpdater, UpdateChannel, UpdateInfo};
