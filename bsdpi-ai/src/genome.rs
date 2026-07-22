//! StrategyGenome — полный набор параметров DPI bypass стратегии.
//!
//! Содержит 50+ полей, описывающих все параметры DPI движка (Zapret, ByeDPI, Warp).
//! Конвертируется в EngineProfile для запуска движка.
//!
//! ## C# оригинал
//! `BSDPI.AI/Models/StrategyGenome.cs`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Тип DPI движка
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

impl std::fmt::Display for DpiEngineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Происхождение стратегии
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyOrigin {
    Builtin,
    Evolved,
    Imported,
    Manual,
}

impl std::fmt::Display for StrategyOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrategyOrigin::Builtin => write!(f, "builtin"),
            StrategyOrigin::Evolved => write!(f, "evolved"),
            StrategyOrigin::Imported => write!(f, "imported"),
            StrategyOrigin::Manual => write!(f, "manual"),
        }
    }
}

/// Геном стратегии — полный набор параметров для DPI bypass.
///
/// Соответствует `StrategyGenome` из C# и `EngineProfile` из BSDPI.Core.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyGenome {
    /// Уникальный ID
    pub id: Uuid,
    /// ID родительских геномов (для evolved)
    pub parent_ids: Vec<Uuid>,
    /// Номер поколения
    pub generation: u32,
    /// Происхождение
    pub origin: StrategyOrigin,

    // === DPI Engine ===
    /// Тип движка
    pub engine_type: DpiEngineType,

    // === Zapret / ByeDPI общие ===
    pub filter_tcp: String,
    pub filter_udp: String,
    /// Режим десинхронизации (split, fake, fakesplit, disorder, etc.)
    pub desync_mode: String,
    /// Позиция split (числовое значение)
    pub split_pos: Option<i32>,
    /// Позиция split (семантический маркер: host, endhost, midsld, sniext, endsld)
    pub split_pos_semantic: Option<String>,
    pub disorder_pos: Option<String>,
    pub fake_pos: Option<String>,
    pub oob_pos: Option<String>,
    pub disoob_pos: Option<String>,
    pub tlsrec_pos: Option<String>,

    // === Zapret специфичные ===
    pub fake_ttl: Option<i32>,
    pub auto_ttl: bool,
    pub md5sig: Option<bool>,
    pub fake_tls_mod: Option<String>,
    pub fake_sni: Option<String>,
    pub fake_data: Option<String>,
    pub mod_http: Option<String>,
    pub tlsminor: Option<i32>,
    pub hosts: Option<String>,
    pub hostlist: Option<String>,
    pub repeat_count: Option<i32>,
    pub cache_ttl: Option<i32>,

    // === ByeDPI специфичные ===
    pub auto: Option<String>,
    pub timeout: Option<i32>,
    pub auto_mode: Option<i32>,

    // === Общие (Zapret + ByeDPI) ===
    pub desync_any_protocol: Option<String>,
    pub desync_fooling: Option<String>,
    pub fake_resend: Option<String>,

    // === Warp специфичные ===
    pub warp_config: Option<String>,
    pub mtu: Option<i32>,
    pub gool_enabled: bool,
    pub psiphon_enabled: bool,
    pub psiphon_country: Option<String>,
    pub scan_enabled: bool,
    pub reserved: Option<String>,

    // === Дополнительные аргументы ===
    pub extra_args: Vec<String>,

    // === Метаданные ===
    pub display_name: String,
    pub bat_file_name: Option<String>,
    pub source_bat_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub orchestrator_enabled: bool,
    pub last_verification_score: Option<i32>,
    pub last_verified_at: Option<DateTime<Utc>>,
}

impl StrategyGenome {
    /// Создаёт новый геном со значениями по умолчанию.
    pub fn new(engine_type: DpiEngineType, origin: StrategyOrigin) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent_ids: Vec::new(),
            generation: 0,
            origin,
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
            display_name: String::new(),
            bat_file_name: None,
            source_bat_path: None,
            created_at: Utc::now(),
            orchestrator_enabled: true,
            last_verification_score: None,
            last_verified_at: None,
        }
    }

    /// Создаёт стандартный геном для Zapret с параметрами по умолчанию.
    pub fn default_zapret() -> Self {
        let mut g = Self::new(DpiEngineType::Zapret, StrategyOrigin::Builtin);
        g.desync_mode = "split".into();
        g.filter_tcp = "80".into();
        g.filter_udp = "443".into();
        g.display_name = "zapret-default".into();
        g
    }

    /// Создаёт стандартный геном для ByeDPI.
    pub fn default_byedpi() -> Self {
        let mut g = Self::new(DpiEngineType::ByeDpi, StrategyOrigin::Builtin);
        g.desync_mode = "split".into();
        g.disorder_pos = Some("1+s".into());
        g.display_name = "byedpi-default".into();
        g
    }

    /// Создаёт стандартный геном для Warp.
    pub fn default_warp() -> Self {
        let mut g = Self::new(DpiEngineType::Warp, StrategyOrigin::Builtin);
        g.mtu = Some(1280);
        g.display_name = "warp-default".into();
        g
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_genome_has_unique_id() {
        let a = StrategyGenome::new(DpiEngineType::Zapret, StrategyOrigin::Builtin);
        let b = StrategyGenome::new(DpiEngineType::Zapret, StrategyOrigin::Builtin);
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn test_default_zapret() {
        let g = StrategyGenome::default_zapret();
        assert_eq!(g.engine_type, DpiEngineType::Zapret);
        assert_eq!(g.origin, StrategyOrigin::Builtin);
        assert_eq!(g.desync_mode, "split");
        assert_eq!(g.filter_tcp, "80");
        assert_eq!(g.filter_udp, "443");
    }

    #[test]
    fn test_default_byedpi() {
        let g = StrategyGenome::default_byedpi();
        assert_eq!(g.engine_type, DpiEngineType::ByeDpi);
        assert_eq!(g.disorder_pos, Some("1+s".into()));
    }

    #[test]
    fn test_default_warp() {
        let g = StrategyGenome::default_warp();
        assert_eq!(g.engine_type, DpiEngineType::Warp);
        assert_eq!(g.mtu, Some(1280));
    }

    #[test]
    fn test_engine_type_display() {
        assert_eq!(DpiEngineType::Zapret.to_string(), "zapret");
        assert_eq!(DpiEngineType::ByeDpi.to_string(), "byedpi");
        assert_eq!(DpiEngineType::Warp.to_string(), "warp");
    }

    #[test]
    fn test_strategy_origin_display() {
        assert_eq!(StrategyOrigin::Builtin.to_string(), "builtin");
        assert_eq!(StrategyOrigin::Evolved.to_string(), "evolved");
    }

    #[test]
    fn test_genome_serialization_roundtrip() {
        let g = StrategyGenome::default_zapret();
        let json = serde_json::to_string(&g).unwrap();
        let deserialized: StrategyGenome = serde_json::from_str(&json).unwrap();
        assert_eq!(g.id, deserialized.id);
        assert_eq!(g.engine_type, deserialized.engine_type);
        assert_eq!(g.desync_mode, deserialized.desync_mode);
    }
}
