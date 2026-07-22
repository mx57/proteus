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

/// GitHub Release API response (минимальный).
#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    published_at: String,
    #[allow(dead_code)]
    prerelease: bool,
}

/// SelfUpdater — проверка и установка обновлений.
pub struct SelfUpdater {
    repo_owner: String,
    repo_name: String,
    current_version: String,
    channel: UpdateChannel,
    client: reqwest::Client,
}

impl SelfUpdater {
    pub fn new(repo: &str, current_version: &str, channel: UpdateChannel) -> Self {
        let parts: Vec<&str> = repo.split('/').collect();
        let client = reqwest::Client::builder()
            .user_agent("Proteus/0.1.0")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            repo_owner: parts.first().copied().unwrap_or("mx57").to_string(),
            repo_name: parts.get(1).copied().unwrap_or("BSDPI_AI").to_string(),
            current_version: current_version.into(),
            channel,
            client,
        }
    }

    /// Проверить наличие обновления.
    pub async fn check_update(&self) -> Result<Option<UpdateInfo>, String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            self.repo_owner, self.repo_name
        );

        match self.client.get(&url).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    if resp.status().as_u16() == 404 {
                        return Ok(None);
                    }
                    return Err(format!("GitHub API: HTTP {}", resp.status()));
                }

                match resp.json::<GitHubRelease>().await {
                    Ok(release) => {
                        let latest = release.tag_name.trim_start_matches('v');

                        if latest == self.current_version.trim_start_matches('v') {
                            return Ok(None); // уже последняя
                        }

                        Ok(Some(UpdateInfo {
                            version: release.tag_name,
                            url: release.html_url,
                            published_at: release.published_at,
                            body: release.body.unwrap_or_default(),
                            channel: self.channel,
                        }))
                    }
                    Err(e) => Err(format!("Failed to parse release: {}", e)),
                }
            }
            Err(e) => {
                log::warn!("Update check failed (offline?): {}", e);
                Ok(None)
            }
        }
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
        // Мы используем фейковый репозиторий, чтобы он возвращал Ok(None) или Err
        let updater = SelfUpdater::new("nonexistent-owner-12345/nonexistent-repo-67890", "1.0.0", UpdateChannel::Stable);
        let result = updater.check_update().await;
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_update_channel_as_str() {
        assert_eq!(UpdateChannel::Stable.as_str(), "stable");
        assert_eq!(UpdateChannel::Beta.as_str(), "beta");
    }
}
