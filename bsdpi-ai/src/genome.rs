//! StrategyGenome — геном стратегии DPI-обхода.
//!
//! Содержит 50+ параметров конфигурации DPI-движков.
//! Может быть сериализован в JSON и преобразован в EngineProfile для запуска.
//!
//! C# оригинал: `BSDPI.AI/Models/StrategyGenome.cs`

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::fmt;

/// Тип DPI-движка.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum DpiEngineType {
    Zapret,
    ByeDpi,
    Warp,
    Hybrid,
    Chained,
    None,
}

impl DpiEngineType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DpiEngineType::Zapret => "zapret",
            DpiEngineType::ByeDpi => "byedpi",
            DpiEngineType::Warp => "warp",
            DpiEngineType::Hybrid => "hybrid",
            DpiEngineType::Chained => "chained",
            DpiEngineType::None => "none",
        }
    }
}

impl fmt::Display for DpiEngineType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Происхождение стратегии.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyOrigin {
    Builtin,
    Evolved,
    Imported,
    Manual,
}

impl StrategyOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            StrategyOrigin::Builtin => "builtin",
            StrategyOrigin::Evolved => "evolved",
            StrategyOrigin::Imported => "imported",
            StrategyOrigin::Manual => "manual",
        }
    }
}

/// Профиль DPI-движка — готовый набор аргументов для запуска.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Собирает CLI аргументы для запуска DPI-движка.
    pub fn to_cli_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        args.push("--dpi-desync".into());
        args.push(self.desync_mode.clone());

        if !self.filter_tcp.is_empty() {
            args.push("--filter-tcp".into());
            args.push(self.filter_tcp.clone());
        }
        if !self.filter_udp.is_empty() {
            args.push("--filter-udp".into());
            args.push(self.filter_udp.clone());
        }
        if let Some(ref v) = self.split_pos { args.push("--dpi-desync-split-pos".into()); args.push(v.clone()); }
        if let Some(ref v) = self.disorder_pos { args.push("--dpi-desync-disorder-pos".into()); args.push(v.clone()); }
        if let Some(ref v) = self.fake_pos { args.push("--dpi-desync-fake-pos".into()); args.push(v.clone()); }
        if let Some(ref v) = self.oob_pos { args.push("--dpi-desync-oob-pos".into()); args.push(v.clone()); }
        if let Some(ref v) = self.disoob_pos { args.push("--dpi-desync-disoob-pos".into()); args.push(v.clone()); }
        if let Some(ref v) = self.tlsrec_pos { args.push("--dpi-desync-tlsrec-pos".into()); args.push(v.clone()); }
        if let Some(v) = self.fake_ttl { args.push("--dpi-desync-ttl".into()); args.push(v.to_string()); }
        if self.auto_ttl { args.push("--dpi-desync-autottl".into()); }
        if let Some(true) = self.md5sig { args.push("--dpi-desync-md5sig".into()); }
        if let Some(ref v) = self.fake_tls_mod { args.push("--dpi-desync-fake-tls-mod".into()); args.push(v.clone()); }
        if let Some(ref v) = self.desync_fooling { args.push("--dpi-desync-fooling".into()); args.push(v.clone()); }
        if let Some(v) = self.repeat_count { args.push("--dpi-desync-repeats".into()); args.push(v.to_string()); }
        if let Some(ref v) = self.hostlist { args.push("--hostlist".into()); args.push(v.clone()); }

        for extra in &self.extra_args {
            if !extra.is_empty() {
                args.push(extra.clone());
            }
        }

        args
    }
}

/// Геном стратегии DPI-обхода.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyGenome {
    pub id: String,
    pub parent_ids: Vec<String>,
    pub generation: u32,
    pub origin: StrategyOrigin,
    pub engine_type: DpiEngineType,

    // DPI parameters
    pub filter_tcp: String,
    pub filter_udp: String,
    pub desync_mode: String,

    pub split_pos: Option<u32>,
    pub split_pos_semantic: Option<String>,
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

    // Metadata
    pub display_name: String,
    pub bat_file_name: Option<String>,
    pub source_bat_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub orchestrator_enabled: bool,
    pub last_verification_score: Option<i32>,
    pub last_verified_at: Option<DateTime<Utc>>,
}

impl StrategyGenome {
    /// Создаёт новый геном с дефолтными значениями.
    pub fn new(engine_type: DpiEngineType, display_name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            parent_ids: Vec::new(),
            generation: 0,
            origin: StrategyOrigin::Builtin,
            engine_type,
            filter_tcp: String::new(),
            filter_udp: String::new(),
            desync_mode: "split".into(),
            split_pos: None,
            split_pos_semantic: None,
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
            display_name,
            bat_file_name: None,
            source_bat_path: None,
            created_at: Utc::now(),
            orchestrator_enabled: true,
            last_verification_score: None,
            last_verified_at: None,
        }
    }

    /// Преобразует геном в EngineProfile для запуска DPI-движка.
    pub fn to_engine_profile(&self, socks_port: u16) -> EngineProfile {
        EngineProfile {
            engine_type: self.engine_type,
            socks_port,
            filter_tcp: self.filter_tcp.clone(),
            filter_udp: self.filter_udp.clone(),
            desync_mode: self.desync_mode.clone(),
            split_pos: self.split_pos_semantic.clone()
                .or_else(|| self.split_pos.map(|v| v.to_string())),
            disorder_pos: self.disorder_pos.clone(),
            fake_pos: self.fake_pos.clone(),
            oob_pos: self.oob_pos.clone(),
            disoob_pos: self.disoob_pos.clone(),
            tlsrec_pos: self.tlsrec_pos.clone(),
            fake_ttl: self.fake_ttl,
            auto_ttl: self.auto_ttl,
            md5sig: self.md5sig,
            fake_tls_mod: self.fake_tls_mod.clone(),
            fake_sni: self.fake_sni.clone(),
            fake_data: self.fake_data.clone(),
            mod_http: self.mod_http.clone(),
            tlsminor: self.tlsminor,
            hosts: self.hosts.clone(),
            hostlist: self.hostlist.clone(),
            repeat_count: self.repeat_count,
            cache_ttl: self.cache_ttl,
            auto: self.auto.clone(),
            timeout: self.timeout,
            auto_mode: self.auto_mode,
            desync_any_protocol: self.desync_any_protocol.clone(),
            desync_fooling: self.desync_fooling.clone(),
            fake_resend: self.fake_resend.clone(),
            warp_config: self.warp_config.clone(),
            mtu: self.mtu,
            gool_enabled: self.gool_enabled,
            psiphon_enabled: self.psiphon_enabled,
            psiphon_country: self.psiphon_country.clone(),
            scan_enabled: self.scan_enabled,
            reserved: self.reserved.clone(),
            extra_args: self.extra_args.clone(),
        }
    }
}

impl fmt::Display for StrategyGenome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Genome[{} | engine={} | gen={} | {:?}]",
            self.display_name, self.engine_type, self.generation, self.origin
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genome_creation() {
        let g = StrategyGenome::new(DpiEngineType::Zapret, "TestGenome".into());
        assert_eq!(g.engine_type, DpiEngineType::Zapret);
        assert_eq!(g.display_name, "TestGenome");
        assert_eq!(g.generation, 0);
        assert_eq!(g.origin, StrategyOrigin::Builtin);
        assert!(!g.id.is_empty());
    }

    #[test]
    fn test_genome_to_engine_profile() {
        let mut g = StrategyGenome::new(DpiEngineType::Zapret, "Test".into());
        g.filter_tcp = "443".into();
        g.filter_udp = "443".into();
        g.desync_mode = "fake".into();
        g.fake_ttl = Some(64);
        g.auto_ttl = true;
        g.repeat_count = Some(3);

        let profile = g.to_engine_profile(1080);
        assert_eq!(profile.engine_type, DpiEngineType::Zapret);
        assert_eq!(profile.socks_port, 1080);
        assert_eq!(profile.filter_tcp, "443");
        assert_eq!(profile.fake_ttl, Some(64));
        assert!(profile.auto_ttl);
    }

    #[test]
    fn test_engine_profile_cli_args() {
        let mut g = StrategyGenome::new(DpiEngineType::Zapret, "Test".into());
        g.desync_mode = "split".into();
        g.filter_tcp = "443".into();
        g.auto_ttl = true;
        g.disorder_pos = Some("3".into());
        g.repeat_count = Some(2);

        let profile = g.to_engine_profile(1080);
        let args = profile.to_cli_args();

        assert!(args.contains(&"--dpi-desync".into()));
        assert!(args.contains(&"split".into()));
        assert!(args.contains(&"--filter-tcp".into()));
        assert!(args.contains(&"443".into()));
        assert!(args.contains(&"--dpi-desync-autottl".into()));
        assert!(args.contains(&"--dpi-desync-disorder-pos".into()));
        assert!(args.contains(&"--dpi-desync-repeats".into()));
    }

    #[test]
    fn test_genome_display() {
        let g = StrategyGenome::new(DpiEngineType::ByeDpi, "MyStrategy".into());
        let display = format!("{}", g);
        assert!(display.contains("MyStrategy"));
        assert!(display.contains("byedpi"));
    }

    #[test]
    fn test_genome_unique_ids() {
        let g1 = StrategyGenome::new(DpiEngineType::Zapret, "A".into());
        let g2 = StrategyGenome::new(DpiEngineType::Zapret, "B".into());
        assert_ne!(g1.id, g2.id);
    }

    #[test]
    fn test_dpi_engine_type_as_str() {
        assert_eq!(DpiEngineType::Zapret.as_str(), "zapret");
        assert_eq!(DpiEngineType::ByeDpi.as_str(), "byedpi");
        assert_eq!(DpiEngineType::Warp.as_str(), "warp");
        assert_eq!(DpiEngineType::None.as_str(), "none");
    }
}
