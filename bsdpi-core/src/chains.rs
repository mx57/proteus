//! ChainBuilder — конструктор цепочек режимов DPI.
//!
//! Определяет режимы работы и цепочки переключения между движками.

use serde::{Deserialize, Serialize};

/// Режим работы DPI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainMode {
    /// Только Zapret
    Zapret,
    /// Только ByeDPI
    ByeDpi,
    /// Только Warp
    Warp,
    /// Zapret + ByeDPI параллельно
    Hybrid,
    /// Warp + Zapret параллельно
    WarpZapret,
    /// Warp + ByeDPI параллельно
    WarpByeDpi,
    /// Warp → Zapret (цепочка)
    ChainedWarpZapret,
    /// Warp → ByeDPI (цепочка)
    ChainedWarpByeDpi,
    /// Без защиты
    Bypass,
}

impl ChainMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChainMode::Zapret => "zapret",
            ChainMode::ByeDpi => "byedpi",
            ChainMode::Warp => "warp",
            ChainMode::Hybrid => "hybrid",
            ChainMode::WarpZapret => "warp+zapret",
            ChainMode::WarpByeDpi => "warp+byedpi",
            ChainMode::ChainedWarpZapret => "warp→zapret",
            ChainMode::ChainedWarpByeDpi => "warp→byedpi",
            ChainMode::Bypass => "bypass",
        }
    }

    /// Сложность обхода для провайдера.
    pub fn difficulty(&self) -> &'static str {
        match self {
            ChainMode::Zapret | ChainMode::ByeDpi | ChainMode::Warp => "low",
            ChainMode::Hybrid | ChainMode::WarpZapret | ChainMode::WarpByeDpi => "medium",
            ChainMode::ChainedWarpZapret | ChainMode::ChainedWarpByeDpi => "extreme",
            ChainMode::Bypass => "none",
        }
    }

    /// Использует ли этот режим SOCKS5 прокси.
    pub fn uses_socks5(&self) -> bool {
        matches!(self, ChainMode::ChainedWarpZapret | ChainMode::ChainedWarpByeDpi)
    }
}

/// Звено цепочки.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainLink {
    pub mode: ChainMode,
    pub order: u32,
    pub socks_port: Option<u16>,
}

impl ChainLink {
    pub fn new(mode: ChainMode, order: u32) -> Self {
        Self { mode, order, socks_port: None }
    }
}

/// ChainBuilder — построитель цепочек.
pub struct ChainBuilder;

impl ChainBuilder {
    /// Получить цепочку для режима.
    pub fn build_chain(mode: ChainMode) -> Vec<ChainLink> {
        match mode {
            ChainMode::Zapret => vec![ChainLink::new(ChainMode::Zapret, 1)],
            ChainMode::ByeDpi => vec![ChainLink::new(ChainMode::ByeDpi, 1)],
            ChainMode::Warp => vec![ChainLink::new(ChainMode::Warp, 1)],
            ChainMode::Hybrid => vec![
                ChainLink::new(ChainMode::Zapret, 1),
                ChainLink::new(ChainMode::ByeDpi, 2),
            ],
            ChainMode::WarpZapret => vec![
                ChainLink::new(ChainMode::Warp, 1),
                ChainLink::new(ChainMode::Zapret, 2),
            ],
            ChainMode::WarpByeDpi => vec![
                ChainLink::new(ChainMode::Warp, 1),
                ChainLink::new(ChainMode::ByeDpi, 2),
            ],
            ChainMode::ChainedWarpZapret => vec![
                ChainLink { mode: ChainMode::Warp, order: 1, socks_port: Some(1086) },
                ChainLink { mode: ChainMode::Zapret, order: 2, socks_port: Some(1080) },
            ],
            ChainMode::ChainedWarpByeDpi => vec![
                ChainLink { mode: ChainMode::Warp, order: 1, socks_port: Some(1086) },
                ChainLink { mode: ChainMode::ByeDpi, order: 2, socks_port: Some(1080) },
            ],
            ChainMode::Bypass => vec![],
        }
    }

    /// Все доступные режимы.
    pub fn all_modes() -> Vec<ChainMode> {
        vec![
            ChainMode::Zapret,
            ChainMode::ByeDpi,
            ChainMode::Warp,
            ChainMode::Hybrid,
            ChainMode::WarpZapret,
            ChainMode::WarpByeDpi,
            ChainMode::ChainedWarpZapret,
            ChainMode::ChainedWarpByeDpi,
            ChainMode::Bypass,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_mode_as_str() {
        assert_eq!(ChainMode::Zapret.as_str(), "zapret");
        assert_eq!(ChainMode::Hybrid.as_str(), "hybrid");
        assert_eq!(ChainMode::Bypass.as_str(), "bypass");
    }

    #[test]
    fn test_chain_difficulty() {
        assert_eq!(ChainMode::Zapret.difficulty(), "low");
        assert_eq!(ChainMode::Hybrid.difficulty(), "medium");
        assert_eq!(ChainMode::ChainedWarpZapret.difficulty(), "extreme");
    }

    #[test]
    fn test_build_single_chain() {
        let chain = ChainBuilder::build_chain(ChainMode::Zapret);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].mode, ChainMode::Zapret);
    }

    #[test]
    fn test_build_hybrid_chain() {
        let chain = ChainBuilder::build_chain(ChainMode::Hybrid);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].mode, ChainMode::Zapret);
        assert_eq!(chain[1].mode, ChainMode::ByeDpi);
    }

    #[test]
    fn test_build_bypass_chain() {
        let chain = ChainBuilder::build_chain(ChainMode::Bypass);
        assert!(chain.is_empty());
    }

    #[test]
    fn test_chained_with_socks() {
        let chain = ChainBuilder::build_chain(ChainMode::ChainedWarpZapret);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].socks_port, Some(1086));
        assert_eq!(chain[1].socks_port, Some(1080));
    }

    #[test]
    fn test_all_modes() {
        let modes = ChainBuilder::all_modes();
        assert_eq!(modes.len(), 9);
    }

    #[test]
    fn test_socks5_usage() {
        assert!(ChainMode::ChainedWarpZapret.uses_socks5());
        assert!(!ChainMode::Zapret.uses_socks5());
    }
}
