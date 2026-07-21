//! Состояние приложения BSDPI GUI.

use chrono::{DateTime, Utc};
use std::collections::VecDeque;

/// Вкладки приложения.
pub const TABS: &[&str] = &["main", "ai", "engine", "chains", "settings", "logs"];

/// Состояние приложения.
pub struct AppState {
    pub version: String,
    pub active_tab: String,

    // Engine panel
    pub engine_status: String,
    pub active_engine: String,
    pub startup_time: DateTime<Utc>,

    // AI panel
    pub bandit_arms: Vec<BanditArmEntry>,
    pub fingerprint_hash: String,
    pub evolver_generation: u32,

    // Chains panel
    pub selected_chain: String,

    // Logs panel
    pub logs: VecDeque<LogEntry>,

    // Settings (editable)
    pub socks_port: u16,
    pub auto_start: bool,
    pub check_interval_secs: u64,
    pub evolution_interval_mins: u64,
    pub engine_dir: String,
    pub log_level: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            version: "0.1.0".into(),
            active_tab: "main".into(),
            engine_status: "Stopped".into(),
            active_engine: "none".into(),
            startup_time: Utc::now(),
            bandit_arms: vec![
                BanditArmEntry::new("Strategy-A", 0.85, 42),
                BanditArmEntry::new("Strategy-B", 0.62, 18),
                BanditArmEntry::new("Strategy-C", 0.45, 7),
            ],
            fingerprint_hash: "a1b2c3d4e5...".into(),
            evolver_generation: 42,
            selected_chain: "Zapret".into(),
            logs: VecDeque::new(),
            socks_port: 1080,
            auto_start: true,
            check_interval_secs: 30,
            evolution_interval_mins: 60,
            engine_dir: "engine".into(),
            log_level: "info".into(),
        }
    }

    pub fn uptime_str(&self) -> String {
        let elapsed = Utc::now() - self.startup_time;
        let secs = elapsed.num_seconds();
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        format!("{:02}:{:02}:{:02}", h, m, s)
    }

    pub fn add_log(&mut self, level: &str, msg: String) {
        self.logs.push_back(LogEntry {
            timestamp: Utc::now(),
            level: level.to_string(),
            message: msg,
        });
        if self.logs.len() > 1000 {
            self.logs.pop_front();
        }
    }
}

/// Arm bandit для отображения.
#[derive(Debug, Clone)]
pub struct BanditArmEntry {
    pub name: String,
    pub mean_reward: f64,
    pub pulls: u32,
}

impl BanditArmEntry {
    pub fn new(name: &str, mean: f64, pulls: u32) -> Self {
        Self {
            name: name.into(),
            mean_reward: mean,
            pulls,
        }
    }
}

/// Запись лога.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
}
