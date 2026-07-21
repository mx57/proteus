//! DpiEngine trait — общий интерфейс для всех DPI-движков.
//!
//! Порт C# `BSDPI.Core/Services/IDpiEngine.cs`

use std::fmt;
use tokio::sync::broadcast;

/// Статус DPI-движка.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl fmt::Display for EngineStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            EngineStatus::Stopped => "stopped",
            EngineStatus::Starting => "starting",
            EngineStatus::Running => "running",
            EngineStatus::Failed => "failed",
            EngineStatus::Crashed => "crashed",
        })
    }
}

/// Информация о запущенном процессе движка.
#[derive(Debug, Clone)]
pub struct EngineProcessInfo {
    pub pid: u32,
    pub process_name: String,
    pub status: EngineStatus,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub socks_port: Option<u16>,
}

impl EngineProcessInfo {
    pub fn new(pid: u32, process_name: &str, socks_port: Option<u16>) -> Self {
        Self {
            pid,
            process_name: process_name.to_string(),
            status: EngineStatus::Running,
            started_at: chrono::Utc::now(),
            socks_port,
        }
    }
}

/// Событие от движка.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    StatusChanged(EngineStatus),
    MessageReceived(String),
    Error(String),
}

/// Ошибки DPI-движка.
#[derive(Debug, Clone)]
pub enum EngineError {
    AlreadyRunning,
    NotRunning,
    ExecutableNotFound(String),
    StartFailed(String),
    StopFailed(String),
    ProbeFailed(String),
    Disposed,
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::AlreadyRunning => write!(f, "engine already running"),
            EngineError::NotRunning => write!(f, "engine not running"),
            EngineError::ExecutableNotFound(path) => write!(f, "executable not found: {}", path),
            EngineError::StartFailed(msg) => write!(f, "start failed: {}", msg),
            EngineError::StopFailed(msg) => write!(f, "stop failed: {}", msg),
            EngineError::ProbeFailed(msg) => write!(f, "probe failed: {}", msg),
            EngineError::Disposed => write!(f, "engine disposed"),
        }
    }
}

impl std::error::Error for EngineError {}

/// Тип DPI-движка (переиспользуем из bsdpi-ai если доступно,
/// но здесь определяем свой для независимости крейта).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
pub trait DpiEngine: Send + Sync {
    /// Получить тип движка.
    fn engine_type(&self) -> DpiEngineType;

    /// Отображаемое имя.
    fn display_name(&self) -> &str;

    /// Текущий статус.
    fn status(&self) -> EngineStatus;

    /// Информация о процессе (если запущен).
    fn process_info(&self) -> Option<EngineProcessInfo>;

    /// Получить receiver для событий движка.
    fn events(&self) -> broadcast::Receiver<EngineEvent>;

    /// Запустить движок с заданным профилем.
    fn start(&mut self, profile: &EngineProfile) -> impl std::future::Future<Output = Result<(), EngineError>> + Send;

    /// Остановить движок.
    fn stop(&mut self) -> impl std::future::Future<Output = Result<(), EngineError>> + Send;

    /// Проверить статус движка.
    fn probe(&mut self) -> impl std::future::Future<Output = EngineStatus> + Send;
}

/// Профиль DPI-движка (аргументы командной строки).
#[derive(Debug, Clone)]
pub struct EngineProfile {
    pub engine_type: DpiEngineType,
    pub socks_port: u16,
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
        if !self.filter_tcp.is_empty() { args.push("--filter-tcp".into()); args.push(self.filter_tcp.clone()); }
        if !self.filter_udp.is_empty() { args.push("--filter-udp".into()); args.push(self.filter_udp.clone()); }
        if !self.desync_mode.is_empty() { args.push("--dpi-desync".into()); args.push(self.desync_mode.clone()); }
        if let Some(ref v) = self.split_pos { args.push("--dpi-desync-split-pos".into()); args.push(v.clone()); }
        if let Some(ref v) = self.disorder_pos { args.push("--dpi-desync-disorder-pos".into()); args.push(v.clone()); }
        if let Some(ref v) = self.fake_pos { args.push("--dpi-desync-fake-pos".into()); args.push(v.clone()); }
        if let Some(ref v) = self.oob_pos { args.push("--dpi-desync-oob-pos".into()); args.push(v.clone()); }
        if let Some(ref v) = self.disoob_pos { args.push("--dpi-desync-disoob-pos".into()); args.push(v.clone()); }
        if let Some(ref v) = self.tlsrec_pos { args.push("--dpi-desync-tlsrec-pos".into()); args.push(v.clone()); }
        if let Some(v) = self.fake_ttl { args.push("--dpi-desync-ttl".into()); args.push(v.to_string()); }
        if self.auto_ttl { args.push("--dpi-desync-autottl".into()); }
        if let Some(ref v) = self.fake_tls_mod { args.push("--dpi-desync-fake-tls-mod".into()); args.push(v.clone()); }
        if let Some(ref v) = self.desync_fooling { args.push("--dpi-desync-fooling".into()); args.push(v.clone()); }
        if let Some(v) = self.repeat_count { args.push("--dpi-desync-repeats".into()); args.push(v.to_string()); }
        if let Some(ref v) = self.hostlist { args.push("--hostlist".into()); args.push(v.clone()); }
        for extra in &self.extra_args { if !extra.is_empty() { args.push(extra.clone()); } }
        args.push("--new".into());
        args
    }

    /// Собрать CLI аргументы для ByeDPI.
    pub fn to_byedpi_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        args.push("-p".into());
        args.push(self.socks_port.to_string());
        if let Some(ref v) = self.split_pos { args.push("--split".into()); args.push(v.clone()); }
        if let Some(ref v) = self.disorder_pos { args.push("--disorder".into()); args.push(v.clone()); }
        if let Some(ref v) = self.fake_pos { args.push("--fake".into()); args.push(v.clone()); }
        if let Some(ref v) = self.oob_pos { args.push("--oob".into()); args.push(v.clone()); }
        if let Some(ref v) = self.disoob_pos { args.push("--disoob".into()); args.push(v.clone()); }
        if let Some(ref v) = self.tlsrec_pos { args.push("--tlsrec".into()); args.push(v.clone()); }
        if let Some(v) = self.fake_ttl { args.push("--ttl".into()); args.push(v.to_string()); }
        if let Some(true) = self.md5sig { args.push("--md5sig".into()); }
        if let Some(ref v) = self.fake_tls_mod { args.push("--fake-tls-mod".into()); args.push(v.clone()); }
        if let Some(ref v) = self.fake_sni { args.push("--fake-sni".into()); args.push(v.clone()); }
        if let Some(ref v) = self.fake_data { args.push("--fake-data".into()); args.push(v.clone()); }
        if let Some(ref v) = self.mod_http { args.push("--mod-http".into()); args.push(v.clone()); }
        if let Some(v) = self.tlsminor { args.push("--tlsminor".into()); args.push(v.to_string()); }
        if let Some(ref v) = self.hosts { args.push("--hosts".into()); args.push(v.clone()); }
        if let Some(v) = self.cache_ttl { args.push("--cache-ttl".into()); args.push(v.to_string()); }
        if let Some(ref v) = self.auto { args.push("--auto".into()); args.push(v.clone()); }
        if let Some(v) = self.timeout { args.push("--timeout".into()); args.push(v.to_string()); }
        if let Some(v) = self.auto_mode { args.push("--auto-mode".into()); args.push(v.to_string()); }
        for extra in &self.extra_args { if !extra.is_empty() { args.push(extra.clone()); } }
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
