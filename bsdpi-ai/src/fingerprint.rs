//! Network Fingerprint — идентификация сети по её характеристикам.
//!
//! Собирает слепок сети: шлюз, DNS, подсеть, транспорт. Хеширует в SHA256.
//! Позволяет хранить отдельную AI-политику для каждой сети (Wi-Fi, мобильный интернет).
//!
//! C# оригинал: `BSDPI.AI/Models/NetworkFingerprint.cs`
//!              `BSDPI.AI/Services/NetworkFingerprintProvider.cs`

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use chrono::{DateTime, Utc};
use std::fmt;

/// Слепок сети — уникальная идентификация сетевого окружения.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkFingerprint {
    /// SHA256 хеш слепка (вычисляется автоматически)
    pub hash: String,
    /// Человеческое название сети (SSID или "Ethernet")
    pub label: String,
    /// Тип транспорта: "WiFi", "Ethernet", "Mobile", "Unknown"
    pub transport: String,
    /// IP шлюза
    pub gateway_ip: String,
    /// Список DNS серверов
    pub dns_servers: Vec<String>,
    /// Локальная подсеть (CIDR)
    pub local_subnet: String,
    /// Время захвата слепка
    pub captured_at: DateTime<Utc>,
}

impl NetworkFingerprint {
    /// Создаёт новый NetworkFingerprint и вычисляет его хеш.
    pub fn new(
        label: String,
        transport: String,
        gateway_ip: String,
        dns_servers: Vec<String>,
        local_subnet: String,
    ) -> Self {
        let mut fp = Self {
            hash: String::new(),
            label,
            transport,
            gateway_ip,
            dns_servers,
            local_subnet,
            captured_at: Utc::now(),
        };
        fp.hash = fp.compute_hash();
        fp
    }

    /// Вычисляет SHA256 хеш от содержимого слепка.
    fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.label.as_bytes());
        hasher.update(self.transport.as_bytes());
        hasher.update(self.gateway_ip.as_bytes());
        for dns in &self.dns_servers {
            hasher.update(dns.as_bytes());
        }
        hasher.update(self.local_subnet.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Проверяет, соответствует ли сетевое окружение этому слепку.
    pub fn matches(&self, other: &NetworkFingerprint) -> bool {
        self.hash == other.hash
    }
}

impl fmt::Display for NetworkFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Network[{}: {} | gw={} | dns={} | subnet={}]",
            self.transport,
            self.label,
            self.gateway_ip,
            self.dns_servers.join(","),
            self.local_subnet
        )
    }
}

/// Провайдер слепков сети — получает текущую информацию о сети.
pub trait FingerprintProvider: Send + Sync {
    /// Получить текущий слепок сети.
    fn current_fingerprint(&self) -> Result<NetworkFingerprint, FingerprintError>;
}

/// Ошибки получения слепка сети.
#[derive(Debug, Clone)]
pub enum FingerprintError {
    NoNetworkInterface,
    NoGateway,
    NoDnsServers,
    Other(String),
}

impl fmt::Display for FingerprintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FingerprintError::NoNetworkInterface => write!(f, "no network interface found"),
            FingerprintError::NoGateway => write!(f, "no gateway found"),
            FingerprintError::NoDnsServers => write!(f, "no DNS servers found"),
            FingerprintError::Other(msg) => write!(f, "fingerprint error: {}", msg),
        }
    }
}

impl std::error::Error for FingerprintError {}

/// Платформонезависимая реализация FingerprintProvider.
/// Использует системные вызовы для получения информации о сети.
/// На Windows — GetAdaptersAddresses / ipconfig
/// На Linux — /proc/net/route, /etc/resolv.conf
/// На Android — /proc/net/route, getprop, dumpsys netstats
pub struct FingerprintProviderImpl;

impl FingerprintProviderImpl {
    /// Создаёт новый провайдер слепков.
    pub fn new() -> Self {
        Self
    }
}

impl Default for FingerprintProviderImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl FingerprintProvider for FingerprintProviderImpl {
    fn current_fingerprint(&self) -> Result<NetworkFingerprint, FingerprintError> {
        // Базовая заглушка — возвращает тестовый слепок.
        // В реальной реализации парсит /proc/net/route, resolv.conf, etc.
        // TODO: platform-specific implementation
        Ok(NetworkFingerprint::new(
            "Local Network".into(),
            "Unknown".into(),
            "0.0.0.0".into(),
            vec!["8.8.8.8".into(), "1.1.1.1".into()],
            "0.0.0.0/0".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_creation() {
        let fp = NetworkFingerprint::new(
            "Home WiFi".into(),
            "WiFi".into(),
            "192.168.1.1".into(),
            vec!["192.168.1.1".into(), "8.8.8.8".into()],
            "192.168.1.0/24".into(),
        );

        assert_eq!(fp.label, "Home WiFi");
        assert_eq!(fp.transport, "WiFi");
        assert_eq!(fp.gateway_ip, "192.168.1.1");
        assert!(!fp.hash.is_empty());
        assert!(fp.hash.len() == 64); // SHA256 hex
    }

    #[test]
    fn test_fingerprint_hash_consistency() {
        let fp1 = NetworkFingerprint::new(
            "Test".into(), "Ethernet".into(),
            "10.0.0.1".into(), vec!["10.0.0.1".into()],
            "10.0.0.0/8".into(),
        );
        let fp2 = NetworkFingerprint::new(
            "Test".into(), "Ethernet".into(),
            "10.0.0.1".into(), vec!["10.0.0.1".into()],
            "10.0.0.0/8".into(),
        );

        assert_eq!(fp1.hash, fp2.hash, "same content should produce same hash");
    }

    #[test]
    fn test_fingerprint_different_hashes() {
        let fp1 = NetworkFingerprint::new(
            "Home".into(), "WiFi".into(),
            "192.168.1.1".into(), vec![].into(),
            "192.168.1.0/24".into(),
        );
        let fp2 = NetworkFingerprint::new(
            "Office".into(), "Ethernet".into(),
            "10.0.0.1".into(), vec![].into(),
            "10.0.0.0/8".into(),
        );

        assert_ne!(fp1.hash, fp2.hash, "different networks should have different hashes");
    }

    #[test]
    fn test_fingerprint_matches() {
        let fp1 = NetworkFingerprint::new(
            "Net".into(), "WiFi".into(),
            "192.168.1.1".into(), vec![].into(),
            "192.168.1.0/24".into(),
        );
        let fp2 = NetworkFingerprint::new(
            "Net".into(), "WiFi".into(),
            "192.168.1.1".into(), vec![].into(),
            "192.168.1.0/24".into(),
        );

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn test_fingerprint_display() {
        let fp = NetworkFingerprint::new(
            "MyNet".into(), "WiFi".into(),
            "192.168.1.1".into(), vec!["8.8.8.8".into()],
            "192.168.1.0/24".into(),
        );
        let display = format!("{}", fp);
        assert!(display.contains("MyNet"));
        assert!(display.contains("WiFi"));
        assert!(display.contains("192.168.1.1"));
    }

    #[test]
    fn test_default_provider() {
        let provider = FingerprintProviderImpl::new();
        let result = provider.current_fingerprint();
        assert!(result.is_ok(), "default provider should always succeed: {:?}", result);
    }
}
