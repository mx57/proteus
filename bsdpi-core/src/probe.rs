//! ProbeService — проверка доступности целей (HTTP, DNS, TCP).
//!
//! Порт C# `BSDPI.Core/Services/ProfileProbeService.cs`
//! + `IConnectivityChecker`, `ProfileProbeResult`, `ProfileProbeOptions`

use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Цель для проверки.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetEntry {
    /// Ключ цели (домен или IP)
    pub key: String,
    /// Тип проверки: "http", "dns", "tcp"
    pub check_type: String,
    /// Порт для TCP проверки
    pub port: Option<u16>,
}

impl TargetEntry {
    pub fn http(domain: &str) -> Self {
        Self { key: domain.into(), check_type: "http".into(), port: None }
    }
    pub fn dns(domain: &str) -> Self {
        Self { key: domain.into(), check_type: "dns".into(), port: None }
    }
    pub fn tcp(host: &str, port: u16) -> Self {
        Self { key: host.into(), check_type: "tcp".into(), port: Some(port) }
    }
}

/// Результат проверки одной цели.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub key: String,
    pub ok: bool,
    pub latency_ms: f64,
    pub error: Option<String>,
}

impl CheckResult {
    pub fn success(key: &str, latency_ms: f64) -> Self {
        Self { key: key.into(), ok: true, latency_ms, error: None }
    }
    pub fn failure(key: &str, error: &str) -> Self {
        Self { key: key.into(), ok: false, latency_ms: 0.0, error: Some(error.into()) }
    }
}

/// Опции проверки.
#[derive(Debug, Clone)]
pub struct ProbeOptions {
    /// Максимальное количество параллельных проверок
    pub max_parallel: usize,
    /// Таймаут на одну проверку
    pub timeout_secs: u64,
    /// Использовать SOCKS5 прокси
    pub socks5_host: Option<String>,
    pub socks5_port: Option<u16>,
    /// Ожидание стабилизации процесса (ms)
    pub stable_wait_ms: u64,
}

impl Default for ProbeOptions {
    fn default() -> Self {
        Self {
            max_parallel: 6,
            timeout_secs: 5,
            socks5_host: None,
            socks5_port: None,
            stable_wait_ms: 500,
        }
    }
}

/// Результат проверки.
#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub success_rate: f64,
    pub checks: Vec<CheckResult>,
    pub total_checks: usize,
    pub passed_checks: usize,
    pub duration_ms: f64,
    pub summary: String,
}

impl ProbeResult {
    pub fn score(&self) -> u32 {
        if self.total_checks == 0 { return 0; }
        (self.success_rate * 100.0) as u32
    }
}

/// ProbeService — проверка доступности.
pub struct ProbeService {
    client: reqwest::Client,
}

impl ProbeService {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .no_proxy()
            .build()
            .expect("Failed to build reqwest::Client");
        Self { client }
    }

    /// Проверить список целей.
    pub async fn check_all(&self, targets: &[TargetEntry], options: &ProbeOptions) -> ProbeResult {
        let start = std::time::Instant::now();

        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(options.max_parallel));
        let timeout = Duration::from_secs(options.timeout_secs);

        let mut handles = Vec::new();
        for target in targets {
            let client = self.client.clone();
            let target = target.clone();
            let permit = semaphore.clone().acquire_owned();

            handles.push(tokio::spawn(async move {
                let _permit = permit.await.unwrap();
                tokio::time::timeout(timeout, Self::check_target(&client, &target)).await
                    .unwrap_or_else(|_| CheckResult::failure(&target.key, "timeout"))
            }));
        }

        let mut results = Vec::new();
        for h in handles {
            if let Ok(r) = h.await {
                results.push(r);
            }
        }

        let elapsed = start.elapsed();
        let passed = results.iter().filter(|r| r.ok).count();
        let total = results.len();
        let rate = if total > 0 { passed as f64 / total as f64 } else { 0.0 };

        let summary = Self::build_summary(passed, total, &results);

        ProbeResult {
            success_rate: rate,
            checks: results,
            total_checks: total,
            passed_checks: passed,
            duration_ms: elapsed.as_secs_f64() * 1000.0,
            summary,
        }
    }

    /// Проверить цели через SOCKS5 прокси.
    pub async fn check_via_socks5(
        &self,
        targets: &[TargetEntry],
        socks_host: &str,
        socks_port: u16,
        options: &ProbeOptions,
    ) -> ProbeResult {
        // SOCKS5 через reqwest proxy
        let proxy_url = format!("socks5://{}:{}", socks_host, socks_port);
        let proxy = match reqwest::Proxy::all(&proxy_url) {
            Ok(p) => p,
            Err(_) => {
                // fallback: без прокси
                let opts = ProbeOptions { socks5_host: None, socks5_port: None, ..options.clone() };
                return self.check_all(targets, &opts).await;
            }
        };
        let client = reqwest::Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap_or_default();

        let opts = ProbeOptions { socks5_host: Some(socks_host.into()), socks5_port: Some(socks_port), ..options.clone() };
        let start = std::time::Instant::now();

        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(opts.max_parallel));
        let mut handles = Vec::new();
        for target in targets {
            let cl = client.clone();
            let t = target.clone();
            let permit = semaphore.clone().acquire_owned();
            handles.push(tokio::spawn(async move {
                let _permit = permit.await.unwrap();
                Self::check_target(&cl, &t).await
            }));
        }

        let mut results = Vec::new();
        for h in handles {
            if let Ok(r) = h.await { results.push(r); }
        }

        let elapsed = start.elapsed();
        let passed = results.iter().filter(|r| r.ok).count();
        let total = results.len();
        let rate = if total > 0 { passed as f64 / total as f64 } else { 0.0 };

        let summary = Self::build_summary(passed, total, &results);

        ProbeResult {
            success_rate: rate,
            checks: results,
            total_checks: total,
            passed_checks: passed,
            duration_ms: elapsed.as_secs_f64() * 1000.0,
            summary,
        }
    }

    async fn check_target(client: &reqwest::Client, target: &TargetEntry) -> CheckResult {
        let start = std::time::Instant::now();
        match target.check_type.as_str() {
            "http" => {
                let url = format!("https://{}/", target.key);
                match client.get(&url).send().await {
                    Ok(resp) => {
                        let latency = start.elapsed().as_secs_f64() * 1000.0;
                        if resp.status().is_success() || resp.status().is_redirection() || resp.status().as_u16() == 403 {
                            CheckResult::success(&target.key, latency)
                        } else {
                            CheckResult::failure(&target.key, &format!("HTTP {}", resp.status()))
                        }
                    }
                    Err(e) => CheckResult::failure(&target.key, &e.to_string()),
                }
            }
            "dns" => {
                // Простая DNS проверка через TCP/IP
                match tokio::net::lookup_host((target.key.as_str(), 0)).await {
                    Ok(addrs) => {
                        let latency = start.elapsed().as_secs_f64() * 1000.0;
                        if addrs.count() > 0 {
                            CheckResult::success(&target.key, latency)
                        } else {
                            CheckResult::failure(&target.key, "no addresses")
                        }
                    }
                    Err(e) => CheckResult::failure(&target.key, &e.to_string()),
                }
            }
            "tcp" => {
                let port = target.port.unwrap_or(443);
                match tokio::net::TcpStream::connect(format!("{}:{}", target.key, port)).await {
                    Ok(_) => {
                        let latency = start.elapsed().as_secs_f64() * 1000.0;
                        CheckResult::success(&target.key, latency)
                    }
                    Err(e) => CheckResult::failure(&target.key, &e.to_string()),
                }
            }
            _ => CheckResult::failure(&target.key, "unknown check type"),
        }
    }

    fn build_summary(passed: usize, total: usize, checks: &[CheckResult]) -> String {
        if total == 0 { return "no targets".into(); }
        let failed: Vec<&str> = checks.iter().filter(|c| !c.ok).map(|c| c.key.as_str()).take(3).collect();
        if failed.is_empty() {
            format!("all OK ({}/{})", passed, total)
        } else {
            format!("OK {}/{}, failed: {}", passed, total, failed.join(", "))
        }
    }
}

impl Default for ProbeService {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_probe_service_creation() {
        let svc = ProbeService::new();
        let targets = vec![TargetEntry::http("example.com")];
        let result = svc.check_all(&targets, &ProbeOptions::default()).await;
        assert_eq!(result.total_checks, 1);
    }

    #[test]
    fn test_target_entry_http() {
        let t = TargetEntry::http("google.com");
        assert_eq!(t.key, "google.com");
        assert_eq!(t.check_type, "http");
    }

    #[test]
    fn test_target_entry_dns() {
        let t = TargetEntry::dns("cloudflare.com");
        assert_eq!(t.key, "cloudflare.com");
        assert_eq!(t.check_type, "dns");
    }

    #[test]
    fn test_probe_result_score() {
        let r = ProbeResult {
            success_rate: 0.8,
            checks: vec![],
            total_checks: 10,
            passed_checks: 8,
            duration_ms: 100.0,
            summary: "OK 8/10".into(),
        };
        assert_eq!(r.score(), 80);
    }

    #[test]
    fn test_check_result_success() {
        let r = CheckResult::success("test.com", 42.5);
        assert!(r.ok);
        assert!((r.latency_ms - 42.5).abs() < 0.01);
    }

    #[test]
    fn test_check_result_failure() {
        let r = CheckResult::failure("test.com", "timeout");
        assert!(!r.ok);
        assert_eq!(r.error.as_deref(), Some("timeout"));
    }

    #[test]
    fn test_default_options() {
        let opts = ProbeOptions::default();
        assert_eq!(opts.max_parallel, 6);
        assert_eq!(opts.timeout_secs, 5);
    }

    #[tokio::test]
    async fn test_check_tcp_timeout() {
        let client = reqwest::Client::new();
        let target = TargetEntry::tcp("192.0.2.1", 9); // reserved IP, should fail
        let result = ProbeService::check_target(&client, &target).await;
        // May timeout or refuse connection - either way it's a failure
        assert!(!result.ok);
    }
}
