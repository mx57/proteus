//! Общие трейты и структуры для DPI движков.

use serde::{Deserialize, Serialize};

/// Статус работы движка.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineStatus {
    Stopped,
    Starting,
    Running,
    Failed,
    Crashed,
}

impl EngineStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, EngineStatus::Running)
    }

    pub fn is_active(&self) -> bool {
        matches!(self, EngineStatus::Starting | EngineStatus::Running)
    }
}

impl std::fmt::Display for EngineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineStatus::Stopped => write!(f, "stopped"),
            EngineStatus::Starting => write!(f, "starting"),
            EngineStatus::Running => write!(f, "running"),
            EngineStatus::Failed => write!(f, "failed"),
            EngineStatus::Crashed => write!(f, "crashed"),
        }
    }
}

/// События от движка.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineEvent {
    StatusChanged(EngineStatus),
    LogLine(String),
}

/// Ошибки движка.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EngineError {
    #[error("Engine is already running")]
    AlreadyRunning,
    #[error("Failed to start engine: {0}")]
    StartFailed(String),
    #[error("Executable not found: {0}")]
    ExecutableNotFound(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Информация о запущенном процессе.
#[derive(Debug, Clone)]
pub struct EngineProcessInfo {
    pub pid: u32,
    pub process_name: String,
    pub socks_port: Option<u16>,
    pub status: EngineStatus,
}

impl EngineProcessInfo {
    pub fn new(pid: u32, process_name: impl Into<String>, socks_port: Option<u16>) -> Self {
        Self {
            pid,
            process_name: process_name.into(),
            socks_port,
            status: EngineStatus::Running,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DpiEngineType {
    Zapret,
    ByeDpi,
    Warp,
}

impl DpiEngineType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DpiEngineType::Zapret => "zapret",
            DpiEngineType::ByeDpi => "byedpi",
            DpiEngineType::Warp => "warp",
        }
    }
}

/// Интерфейс DPI-движка (порт C# IDpiEngine).
#[async_trait::async_trait]
pub trait DpiEngine: Send + Sync {
    /// Получить тип движка.
    fn engine_type(&self) -> DpiEngineType;

    /// Отображаемое имя (например, "Zapret winws.exe").
    fn display_name(&self) -> &str;

    /// Текущий статус.
    fn status(&self) -> EngineStatus;

    /// Информация о процессе (если запущен).
    fn process_info(&self) -> Option<EngineProcessInfo>;

    /// Канал для подписки на события (статусы, логи).
    fn events(&self) -> tokio::sync::broadcast::Receiver<EngineEvent>;

    /// Запустить движок с указанным профилем.
    async fn start(&mut self, profile: &EngineProfile) -> Result<(), EngineError>;

    /// Остановить движок (мягко или жестко).
    async fn stop(&mut self) -> Result<(), EngineError>;

    /// Принудительно проверить жив ли процесс, возвращает актуальный статус.
    async fn probe(&mut self) -> EngineStatus;
}

/// Платформо-независимые параметры запуска DPI движка.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineProfile {
    pub engine_type: DpiEngineType,
    pub socks_port: u16,

    // Общие параметры обхода
    pub filter_tcp: String,
    pub filter_udp: String,
    pub desync_mode: String,
    pub split_pos: Option<String>,
    pub disorder_pos: Option<String>,
    pub fake_pos: Option<String>,
    pub oob_pos: Option<String>,
    pub disoob_pos: Option<String>,
    pub tlsrec_pos: Option<String>,

    pub fake_ttl: Option<u32>,
    pub auto_ttl: bool,

    pub md5sig: Option<bool>,
    pub fake_tls_mod: Option<String>,
    pub fake_sni: Option<String>,
    pub fake_data: Option<String>,
    pub mod_http: Option<String>,
    pub tlsminor: Option<u32>,
    pub hosts: Option<String>,
    pub hostlist: Option<String>,
    pub repeat_count: Option<u32>,
    pub cache_ttl: Option<u32>,
    pub auto: Option<String>,
    pub timeout: Option<u32>,
    pub auto_mode: Option<u32>,
    pub desync_any_protocol: Option<String>,
    pub desync_fooling: Option<String>,
    pub fake_resend: Option<String>,
    pub warp_config: Option<String>,
    pub mtu: Option<u32>,
    pub gool_enabled: bool,
    pub psiphon_enabled: bool,
    pub psiphon_country: Option<String>,
    pub scan_enabled: bool,
    pub reserved: Option<String>,
    pub extra_args: Vec<String>,
}

impl EngineProfile {
    /// Собрать CLI аргументы для Zapret.
    pub fn to_zapret_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        if !self.filter_tcp.is_empty() {
            args.push("--filter-tcp".into());
            args.push(self.filter_tcp.clone());
        }
        if !self.filter_udp.is_empty() {
            args.push("--filter-udp".into());
            args.push(self.filter_udp.clone());
        }
        if !self.desync_mode.is_empty() {
            args.push("--dpi-desync".into());
            args.push(self.desync_mode.clone());
        }
        if let Some(ref v) = self.split_pos {
            args.push("--dpi-desync-split-pos".into());
            args.push(v.clone());
        }
        if let Some(ref v) = self.disorder_pos {
            args.push("--dpi-desync-disorder-pos".into());
            args.push(v.clone());
        }
        if let Some(ref v) = self.fake_pos {
            args.push("--dpi-desync-fake-pos".into());
            args.push(v.clone());
        }
        if let Some(ref v) = self.oob_pos {
            args.push("--dpi-desync-oob-pos".into());
            args.push(v.clone());
        }
        if let Some(ref v) = self.disoob_pos {
            args.push("--dpi-desync-disoob-pos".into());
            args.push(v.clone());
        }
        if let Some(ref v) = self.tlsrec_pos {
            args.push("--dpi-desync-tlsrec-pos".into());
            args.push(v.clone());
        }
        if let Some(v) = self.fake_ttl {
            args.push("--dpi-desync-ttl".into());
            args.push(v.to_string());
        }
        if self.auto_ttl {
            args.push("--dpi-desync-autottl".into());
        }
        if let Some(ref v) = self.fake_tls_mod {
            args.push("--dpi-desync-fake-tls-mod".into());
            args.push(v.clone());
        }
        if let Some(ref v) = self.desync_fooling {
            args.push("--dpi-desync-fooling".into());
            args.push(v.clone());
        }
        if let Some(v) = self.repeat_count {
            args.push("--dpi-desync-repeats".into());
            args.push(v.to_string());
        }
        if let Some(ref v) = self.hostlist {
            args.push("--hostlist".into());
            args.push(v.clone());
        }
        for extra in &self.extra_args {
            if !extra.is_empty() {
                args.push(extra.clone());
            }
        }
        args.push("--new".into());
        args
    }

    /// Собрать CLI аргументы для ByeDPI.
    pub fn to_byedpi_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        args.push("-p".into());
        args.push(self.socks_port.to_string());
        if let Some(ref v) = self.split_pos {
            args.push("--split".into());
            args.push(v.clone());
        }
        if let Some(ref v) = self.disorder_pos {
            args.push("--disorder".into());
            args.push(v.clone());
        }
        if let Some(ref v) = self.fake_pos {
            args.push("--fake".into());
            args.push(v.clone());
        }
        if let Some(ref v) = self.oob_pos {
            args.push("--oob".into());
            args.push(v.clone());
        }
        if let Some(ref v) = self.disoob_pos {
            args.push("--disoob".into());
            args.push(v.clone());
        }
        if let Some(ref v) = self.tlsrec_pos {
            args.push("--tlsrec".into());
            args.push(v.clone());
        }
        if let Some(v) = self.fake_ttl {
            args.push("--ttl".into());
            args.push(v.to_string());
        }
        if let Some(true) = self.md5sig {
            args.push("--md5sig".into());
        }
        if let Some(ref v) = self.fake_tls_mod {
            args.push("--fake-tls-mod".into());
            args.push(v.clone());
        }
        if let Some(ref v) = self.fake_sni {
            args.push("--fake-sni".into());
            args.push(v.clone());
        }
        if let Some(ref v) = self.fake_data {
            args.push("--fake-data".into());
            args.push(v.clone());
        }
        if let Some(ref v) = self.mod_http {
            args.push("--mod-http".into());
            args.push(v.clone());
        }
        if let Some(v) = self.tlsminor {
            args.push("--tlsminor".into());
            args.push(v.to_string());
        }
        if let Some(ref v) = self.hosts {
            args.push("--hosts".into());
            args.push(v.clone());
        }
        if let Some(v) = self.cache_ttl {
            args.push("--cache-ttl".into());
            args.push(v.to_string());
        }
        if let Some(ref v) = self.auto {
            args.push("--auto".into());
            args.push(v.clone());
        }
        if let Some(v) = self.timeout {
            args.push("--timeout".into());
            args.push(v.to_string());
        }
        if let Some(v) = self.auto_mode {
            args.push("--auto-mode".into());
            args.push(v.to_string());
        }
        for extra in &self.extra_args {
            if !extra.is_empty() {
                args.push(extra.clone());
            }
        }
        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_status_display() {
        assert_eq!(EngineStatus::Stopped.to_string(), "stopped");
        assert_eq!(EngineStatus::Running.to_string(), "running");
        assert_eq!(EngineStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_engine_status_is_running() {
        assert!(EngineStatus::Running.is_running());
        assert!(!EngineStatus::Stopped.is_running());
        assert!(!EngineStatus::Failed.is_running());
    }

    #[test]
    fn test_engine_status_is_active() {
        assert!(EngineStatus::Starting.is_active());
        assert!(EngineStatus::Running.is_active());
        assert!(!EngineStatus::Stopped.is_active());
    }

    #[test]
    fn test_dpi_engine_type_as_str() {
        assert_eq!(DpiEngineType::Zapret.as_str(), "zapret");
        assert_eq!(DpiEngineType::ByeDpi.as_str(), "byedpi");
        assert_eq!(DpiEngineType::Warp.as_str(), "warp");
    }

    #[test]
    fn test_engine_process_info() {
        let info = EngineProcessInfo::new(12345, "winws.exe", Some(1080));
        assert_eq!(info.pid, 12345);
        assert_eq!(info.process_name, "winws.exe");
        assert_eq!(info.socks_port, Some(1080));
        assert_eq!(info.status, EngineStatus::Running);
    }

    #[test]
    fn test_zapret_args_basic() {
        let profile = EngineProfile {
            filter_tcp: "443".into(),
            filter_udp: "443".into(),
            desync_mode: "fake".into(),
            fake_ttl: Some(64),
            auto_ttl: true,
            repeat_count: Some(3),
            hostlist: Some("blocked.txt".into()),
            ..Default::default()
        };
        let args = profile.to_zapret_args();
        assert!(args.contains(&"--filter-tcp".into()));
        assert!(args.contains(&"443".into()));
        assert!(args.contains(&"--dpi-desync".into()));
        assert!(args.contains(&"fake".into()));
        assert!(args.contains(&"--dpi-desync-autottl".into()));
        assert!(args.contains(&"--new".into()));
    }

    #[test]
    fn test_byedpi_args_basic() {
        let profile = EngineProfile {
            socks_port: 2080,
            split_pos: Some("3".into()),
            md5sig: Some(true),
            ..Default::default()
        };
        let args = profile.to_byedpi_args();
        assert!(args.contains(&"-p".into()));
        assert!(args.contains(&"2080".into()));
        assert!(args.contains(&"--split".into()));
        assert!(args.contains(&"--md5sig".into()));
    }

    #[test]
    fn test_engine_error_display() {
        let err = EngineError::ExecutableNotFound("/bin/winws".into());
        assert!(err.to_string().contains("/bin/winws"));
        assert!(err.to_string().contains("not found"));
    }
}

impl Default for EngineProfile {
    fn default() -> Self {
        Self {
            engine_type: DpiEngineType::Zapret,
            socks_port: 1080,
            filter_tcp: String::new(),
            filter_udp: String::new(),
            desync_mode: "split".into(),
            split_pos: None,
            disorder_pos: None,
            fake_pos: None,
            oob_pos: None,
            disoob_pos: None,
            tlsrec_pos: None,
            fake_ttl: None,
            auto_ttl: false,
            md5sig: None,
            fake_tls_mod: None,
            fake_sni: None,
            fake_data: None,
            mod_http: None,
            tlsminor: None,
            hosts: None,
            hostlist: None,
            repeat_count: None,
            cache_ttl: None,
            auto: None,
            timeout: None,
            auto_mode: None,
            desync_any_protocol: None,
            desync_fooling: None,
            fake_resend: None,
            warp_config: None,
            mtu: None,
            gool_enabled: false,
            psiphon_enabled: false,
            psiphon_country: None,
            scan_enabled: false,
            reserved: None,
            extra_args: Vec::new(),
        }
    }
}
