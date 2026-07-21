//! SettingsService — TOML-конфиг приложения.

use serde::{Deserialize, Serialize};

/// Настройки приложения.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub engine_dir: String,
    pub socks_port: u16,
    pub auto_start: bool,
    pub check_interval_secs: u64,
    pub evolution_interval_mins: u64,
    pub max_parallel_checks: usize,
    pub probe_timeout_secs: u64,
    pub log_level: String,
    pub auto_update: bool,
    pub update_channel: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            engine_dir: "engine".into(),
            socks_port: 1080,
            auto_start: true,
            check_interval_secs: 30,
            evolution_interval_mins: 60,
            max_parallel_checks: 6,
            probe_timeout_secs: 5,
            log_level: "info".into(),
            auto_update: true,
            update_channel: "stable".into(),
        }
    }
}

/// SettingsService — загрузка и сохранение конфига.
pub struct SettingsService {
    path: String,
    settings: AppSettings,
}

impl SettingsService {
    pub fn new(path: String) -> Self {
        let settings = if std::path::Path::new(&path).exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| toml::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            AppSettings::default()
        };
        Self { path, settings }
    }

    pub fn get(&self) -> &AppSettings { &self.settings }
    pub fn get_mut(&mut self) -> &mut AppSettings { &mut self.settings }

    pub fn save(&self) -> Result<(), String> {
        let toml_str = toml::to_string_pretty(&self.settings).map_err(|e| e.to_string())?;
        std::fs::write(&self.path, &toml_str).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn reload(&mut self) {
        if std::path::Path::new(&self.path).exists() {
            if let Ok(s) = std::fs::read_to_string(&self.path) {
                if let Ok(settings) = toml::from_str(&s) {
                    self.settings = settings;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let s = AppSettings::default();
        assert_eq!(s.socks_port, 1080);
        assert!(s.auto_start);
    }

    #[test]
    fn test_settings_new_with_nonexistent_path() {
        let svc = SettingsService::new("/nonexistent/path.toml".into());
        // Should use defaults
        assert_eq!(svc.get().socks_port, 1080);
    }

    #[test]
    fn test_settings_custom_values() {
        let mut s = AppSettings::default();
        s.socks_port = 2080;
        s.auto_start = false;
        assert_eq!(s.socks_port, 2080);
        assert!(!s.auto_start);
    }

    #[test]
    fn test_save_and_load() {
        let path = "/tmp/bsdpi-test-settings.toml";
        let _ = std::fs::remove_file(path);

        // Save
        {
            let mut svc = SettingsService::new(path.into());
            svc.get_mut().socks_port = 3090;
            svc.save().unwrap();
        }

        // Load
        {
            let svc = SettingsService::new(path.into());
            assert_eq!(svc.get().socks_port, 3090);
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_toml_serialization() {
        let s = AppSettings::default();
        let toml_str = toml::to_string_pretty(&s).unwrap();
        assert!(toml_str.contains("socks_port"));
        assert!(toml_str.contains("engine_dir"));

        let deserialized: AppSettings = toml::from_str(&toml_str).unwrap();
        assert_eq!(deserialized.socks_port, s.socks_port);
    }
}
