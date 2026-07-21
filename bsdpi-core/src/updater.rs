//! SelfUpdater — автообновление через GitHub Releases.

use serde::{Deserialize, Serialize};

/// Канал обновлений.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateChannel {
    Stable,
    Beta,
    Nightly,
}

impl UpdateChannel {
    pub fn as_str(&self) -> &'static str {
        match self {
            UpdateChannel::Stable => "stable",
            UpdateChannel::Beta => "beta",
            UpdateChannel::Nightly => "nightly",
        }
    }
}

/// Информация о доступном обновлении.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
    pub published_at: String,
    pub body: String,
    pub channel: UpdateChannel,
}

/// SelfUpdater — проверка и установка обновлений.
pub struct SelfUpdater {
    repo_owner: String,
    repo_name: String,
    current_version: String,
    channel: UpdateChannel,
}

impl SelfUpdater {
    pub fn new(repo: &str, current_version: &str, channel: UpdateChannel) -> Self {
        let parts: Vec<&str> = repo.split('/').collect();
        Self {
            repo_owner: parts.first().copied().unwrap_or("mx57").to_string(),
            repo_name: parts.get(1).copied().unwrap_or("BSDPI_AI").to_string(),
            current_version: current_version.into(),
            channel,
        }
    }

    /// Проверить наличие обновления (имитация — в реальном коде HTTP запрос к GitHub API).
    pub async fn check_update(&self) -> Result<Option<UpdateInfo>, String> {
        // TODO: реальный запрос к https://api.github.com/repos/{owner}/{repo}/releases/latest
        // Пока возвращаем заглушку
        Ok(None)
    }

    pub fn current_version(&self) -> &str { &self.current_version }
    pub fn channel(&self) -> UpdateChannel { self.channel }
    pub fn repo(&self) -> String { format!("{}/{}", self.repo_owner, self.repo_name) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_updater_creation() {
        let updater = SelfUpdater::new("mx57/BSDPI_AI", "1.0.0", UpdateChannel::Stable);
        assert_eq!(updater.current_version(), "1.0.0");
        assert_eq!(updater.channel(), UpdateChannel::Stable);
        assert_eq!(updater.repo(), "mx57/BSDPI_AI");
    }

    #[tokio::test]
    async fn test_check_update_returns_none() {
        let updater = SelfUpdater::new("mx57/BSDPI_AI", "1.0.0", UpdateChannel::Stable);
        let result = updater.check_update().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_update_channel_as_str() {
        assert_eq!(UpdateChannel::Stable.as_str(), "stable");
        assert_eq!(UpdateChannel::Beta.as_str(), "beta");
    }
}
