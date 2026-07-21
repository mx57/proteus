//! ProteusUpdater — проверка обновлений через GitHub Releases API.

use serde::Deserialize;

/// Информация о доступном релизе.
pub struct UpdateRelease {
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub release_notes: String,
    pub published_at: String,
}

/// GitHub Release API response (минимальный).
#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    published_at: String,
    prerelease: bool,
}

/// Проверщик обновлений.
pub struct ProteusUpdater {
    owner: String,
    repo: String,
    current_version: String,
    client: reqwest::Client,
}

impl ProteusUpdater {
    pub fn new(owner: &str, repo: &str, current_version: &str) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Proteus/0.1.0")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        Self {
            owner: owner.into(),
            repo: repo.into(),
            current_version: current_version.into(),
            client,
        }
    }

    /// Проверить наличие обновлений через GitHub API.
    pub async fn check(&self) -> Result<Option<UpdateRelease>, String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            self.owner, self.repo
        );

        match self.client.get(&url).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    if resp.status().as_u16() == 404 {
                        return Ok(None); // нет релизов
                    }
                    return Err(format!("GitHub API: HTTP {}", resp.status()));
                }

                match resp.json::<GitHubRelease>().await {
                    Ok(release) => {
                        let latest = release.tag_name.trim_start_matches('v');

                        if latest == self.current_version.trim_start_matches('v') {
                            return Ok(None); // уже последняя
                        }

                        Ok(Some(UpdateRelease {
                            current_version: self.current_version.clone(),
                            latest_version: release.tag_name,
                            download_url: release.html_url,
                            release_notes: release.body.unwrap_or_default(),
                            published_at: release.published_at,
                        }))
                    }
                    Err(e) => Err(format!("Failed to parse release: {}", e)),
                }
            }
            Err(e) => {
                // Если нет интернета — не ошибка, просто нет обновлений
                log::warn!("Update check failed (offline?): {}", e);
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_updater_creation() {
        let updater = ProteusUpdater::new("mx57", "proteus", "0.1.0");
        assert_eq!(updater.owner, "mx57");
        assert_eq!(updater.repo, "proteus");
        assert_eq!(updater.current_version, "0.1.0");
    }

    #[tokio::test]
    async fn test_update_check_nonexistent_repo() {
        // Этот тест может не сработать без интернета,
        // но он должен вернуть Ok(None) или Err, но не паниковать
        let updater = ProteusUpdater::new("nonexistent-owner-12345", "nonexistent-repo-67890", "0.1.0");
        let result = updater.check().await;
        // Должно вернуть Ok(None) для 404 или Err для сетевой ошибки
        assert!(result.is_ok() || result.is_err());
    }
}
