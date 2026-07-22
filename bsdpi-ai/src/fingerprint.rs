//! Network fingerprinting — идентификация сети для per-network AI политик.
//!
//! Каждая сеть (Wi-Fi, мобильный интернет) получает уникальный SHA256 хеш,
//! по которому хранится своя политика ИИ.
//!
//! ## C# оригинал
//! - `BSDPI.AI/Models/NetworkFingerprint.cs`
//! - `BSDPI.AI/Services/NetworkFingerprintProvider.cs`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// Отпечаток сети — уникальный идентификатор сетевого окружения.
///
/// Содержит тип сети, шлюз, DNS, подсеть и метку времени.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NetworkFingerprint {
    /// SHA256 хеш отпечатка (уникальный ID сети)
    pub hash: String,
    /// Человеко-читаемая метка (например, "Домашний Wi-Fi")
    pub label: String,
    /// Тип транспорта (WiFi, Cellular, Ethernet)
    pub transport: String,
    /// IP адрес шлюза
    pub gateway_ip: String,
    /// Список DNS серверов
    pub dns_servers: Vec<String>,
    /// Локальная подсеть (CIDR)
    pub local_subnet: String,
    /// Время захвата
    pub captured_at: DateTime<Utc>,
}

impl NetworkFingerprint {
    /// Создаёт новый отпечаток сети, автоматически вычисляя хеш.
    pub fn new(
        label: impl Into<String>,
        transport: impl Into<String>,
        gateway_ip: impl Into<String>,
        dns_servers: Vec<String>,
        local_subnet: impl Into<String>,
    ) -> Self {
        let mut fp = Self {
            hash: String::new(), // will be filled below
            label: label.into(),
            transport: transport.into(),
            gateway_ip: gateway_ip.into(),
            dns_servers,
            local_subnet: local_subnet.into(),
            captured_at: Utc::now(),
        };
        fp.hash = fp.compute_hash();
        fp
    }

    /// Вычисляет SHA256 хеш отпечатка сети.
    fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.transport.as_bytes());
        hasher.update(b"|");
        hasher.update(self.gateway_ip.as_bytes());
        hasher.update(b"|");
        for dns in &self.dns_servers {
            hasher.update(dns.as_bytes());
            hasher.update(b",");
        }
        hasher.update(b"|");
        hasher.update(self.local_subnet.as_bytes());
        hex::encode(hasher.finalize())
    }
}

impl fmt::Display for NetworkFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NetworkFingerprint({} | {} | gateway: {} | subnet: {})",
            self.label, self.transport, self.gateway_ip, self.local_subnet
        )
    }
}

/// Поставщик отпечатков сети — трейт для платформо-зависимой реализации.
///
/// На Windows использует `NetworkInterface.GetAllNetworkInterfaces()`,
/// на Linux — `/proc/net/route` + `ip route`, на Android — `ConnectivityManager`.
pub trait FingerprintProvider: Send + Sync {
    /// Получить текущий отпечаток сети.
    fn current_fingerprint(&self) -> Option<NetworkFingerprint>;

    /// Получить человеко-читаемую метку сети.
    fn network_label(&self) -> String;
}

/// Простая реализация FingerprintProvider для тестов и Linux.
pub struct BasicFingerprintProvider {
    label: String,
    transport: String,
    gateway: String,
    dns: Vec<String>,
    subnet: String,
}

impl BasicFingerprintProvider {
    pub fn new(
        label: impl Into<String>,
        transport: impl Into<String>,
        gateway: impl Into<String>,
        dns: Vec<String>,
        subnet: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            transport: transport.into(),
            gateway: gateway.into(),
            dns,
            subnet: subnet.into(),
        }
    }
}

impl FingerprintProvider for BasicFingerprintProvider {
    fn current_fingerprint(&self) -> Option<NetworkFingerprint> {
        Some(NetworkFingerprint::new(
            &self.label,
            &self.transport,
            &self.gateway,
            self.dns.clone(),
            &self.subnet,
        ))
    }

    fn network_label(&self) -> String {
        self.label.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_creation() {
        let fp = NetworkFingerprint::new(
            "Home WiFi",
            "WiFi",
            "192.168.1.1",
            vec!["8.8.8.8".into(), "1.1.1.1".into()],
            "192.168.1.0/24",
        );
        assert_eq!(fp.label, "Home WiFi");
        assert_eq!(fp.transport, "WiFi");
        assert_eq!(fp.gateway_ip, "192.168.1.1");
        assert_eq!(fp.dns_servers.len(), 2);
        assert_eq!(fp.local_subnet, "192.168.1.0/24");
        assert!(!fp.hash.is_empty());
    }

    #[test]
    fn test_same_network_same_hash() {
        let a = NetworkFingerprint::new(
            "Home",
            "WiFi",
            "192.168.1.1",
            vec!["8.8.8.8".into()],
            "192.168.1.0/24",
        );
        let b = NetworkFingerprint::new(
            "Home (alias)",
            "WiFi",
            "192.168.1.1",
            vec!["8.8.8.8".into()],
            "192.168.1.0/24",
        );
        // Label не влияет на хеш — только сетевые параметры
        assert_eq!(a.hash, b.hash);
    }

    #[test]
    fn test_different_networks_different_hash() {
        let a = NetworkFingerprint::new("Home", "WiFi", "192.168.1.1", vec![], "192.168.1.0/24");
        let b = NetworkFingerprint::new("Mobile", "Cellular", "10.0.0.1", vec![], "10.0.0.0/8");
        assert_ne!(a.hash, b.hash);
    }

    #[test]
    fn test_basic_provider() {
        let provider = BasicFingerprintProvider::new(
            "Test Net",
            "Ethernet",
            "10.0.0.1",
            vec!["10.0.0.53".into()],
            "10.0.0.0/24",
        );
        assert_eq!(provider.network_label(), "Test Net");

        let fp = provider.current_fingerprint().unwrap();
        assert_eq!(fp.gateway_ip, "10.0.0.1");
    }

    #[test]
    fn test_fingerprint_display() {
        let fp = NetworkFingerprint::new(
            "Office",
            "Ethernet",
            "172.16.0.1",
            vec![],
            "172.16.0.0/16",
        );
        let display = format!("{fp}");
        assert!(display.contains("Office"));
        assert!(display.contains("Ethernet"));
        assert!(display.contains("172.16.0.1"));
    }
}
