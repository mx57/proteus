//! GenomeSignature — уникальная сигнатура генома на основе SHA256.
//!
//! Берёт все значимые поля генома, сериализует в JSON (отсортированный),
//! и вычисляет SHA256 хеш. Используется для дедупликации стратегий.
//!
//! C# оригинал: `BSDPI.AI/Models/GenomeSignature.cs`

use crate::genome::StrategyGenome;
use sha2::{Digest, Sha256};

/// Вычисляет уникальную сигнатуру генома.
pub struct GenomeSignature;

impl GenomeSignature {
    /// Вычисляет SHA256 сигнатуру стратегии.
    ///
    /// Сериализует только значимые для сигнатуры поля в отсортированный JSON
    /// (детерминированный порядок ключей) и хеширует SHA256.
    /// Результат: 64-символьная hex строка.
    pub fn compute(genome: &StrategyGenome) -> String {
        // Собираем детерминированное представление для хеширования.
        // Используем record-style формат: поле=значение разделённые \n
        let mut payload = String::new();

        payload.push_str(&format!("engine={}\n", genome.engine_type));
        payload.push_str(&format!("filter_tcp={}\n", genome.filter_tcp));
        payload.push_str(&format!("filter_udp={}\n", genome.filter_udp));
        payload.push_str(&format!("desync_mode={}\n", genome.desync_mode));
        payload.push_str(&format!("split_pos={}\n", opt_u32_str(genome.split_pos)));
        payload.push_str(&format!("split_pos_sem={}\n", opt_str(&genome.split_pos_semantic)));
        payload.push_str(&format!("disorder_pos={}\n", opt_str(&genome.disorder_pos)));
        payload.push_str(&format!("fake_pos={}\n", opt_str(&genome.fake_pos)));
        payload.push_str(&format!("oob_pos={}\n", opt_str(&genome.oob_pos)));
        payload.push_str(&format!("disoob_pos={}\n", opt_str(&genome.disoob_pos)));
        payload.push_str(&format!("tlsrec_pos={}\n", opt_str(&genome.tlsrec_pos)));
        payload.push_str(&format!("fake_ttl={}\n", opt_u32_str(genome.fake_ttl)));
        payload.push_str(&format!("auto_ttl={}\n", genome.auto_ttl));
        payload.push_str(&format!("md5sig={}\n", opt_bool_str(genome.md5sig)));
        payload.push_str(&format!("fake_tls_mod={}\n", opt_str(&genome.fake_tls_mod)));
        payload.push_str(&format!("fake_sni={}\n", opt_str(&genome.fake_sni)));
        payload.push_str(&format!("fake_data={}\n", opt_str(&genome.fake_data)));
        payload.push_str(&format!("mod_http={}\n", opt_str(&genome.mod_http)));
        payload.push_str(&format!("tlsminor={}\n", opt_u32_str(genome.tlsminor)));
        payload.push_str(&format!("hosts={}\n", opt_str(&genome.hosts)));
        payload.push_str(&format!("hostlist={}\n", opt_str(&genome.hostlist)));
        payload.push_str(&format!("repeat_count={}\n", opt_u32_str(genome.repeat_count)));
        payload.push_str(&format!("cache_ttl={}\n", opt_u32_str(genome.cache_ttl)));
        payload.push_str(&format!("auto={}\n", opt_str(&genome.auto)));
        payload.push_str(&format!("timeout={}\n", opt_u32_str(genome.timeout)));
        payload.push_str(&format!("auto_mode={}\n", opt_u32_str(genome.auto_mode)));
        payload.push_str(&format!("desync_any_proto={}\n", opt_str(&genome.desync_any_protocol)));
        payload.push_str(&format!("desync_fooling={}\n", opt_str(&genome.desync_fooling)));
        payload.push_str(&format!("fake_resend={}\n", opt_str(&genome.fake_resend)));
        payload.push_str(&format!("warp_config={}\n", opt_str(&genome.warp_config)));
        payload.push_str(&format!("mtu={}\n", opt_u32_str(genome.mtu)));
        payload.push_str(&format!("gool={}\n", genome.gool_enabled));
        payload.push_str(&format!("psiphon={}\n", genome.psiphon_enabled));
        payload.push_str(&format!("psiphon_country={}\n", opt_str(&genome.psiphon_country)));
        payload.push_str(&format!("scan={}\n", genome.scan_enabled));
        payload.push_str(&format!("reserved={}\n", opt_str(&genome.reserved)));
        payload.push_str(&format!("extra={}\n", genome.extra_args.join("\x1f")));

        let mut hasher = Sha256::new();
        hasher.update(payload.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

fn opt_str(v: &Option<String>) -> &str {
    v.as_deref().unwrap_or("")
}

fn opt_u32_str(v: Option<u32>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}

fn opt_bool_str(v: Option<bool>) -> String {
    match v {
        Some(true) => "true".into(),
        Some(false) => "false".into(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::{DpiEngineType, StrategyGenome};

    #[test]
    fn test_signature_is_deterministic() {
        let g1 = make_test_genome();
        let g2 = make_test_genome();

        let s1 = GenomeSignature::compute(&g1);
        let s2 = GenomeSignature::compute(&g2);

        assert_eq!(s1, s2, "same genome should produce same signature");
    }

    #[test]
    fn test_signature_64_hex_chars() {
        let g = make_test_genome();
        let sig = GenomeSignature::compute(&g);
        assert_eq!(sig.len(), 64, "SHA256 hex should be 64 chars, got {}", sig.len());
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()), "expected hex string");
    }

    #[test]
    fn test_different_genomes_different_signatures() {
        let mut g1 = make_test_genome();
        let mut g2 = make_test_genome();
        g2.desync_mode = "fake".into();

        let s1 = GenomeSignature::compute(&g1);
        let s2 = GenomeSignature::compute(&g2);
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_signature_depends_on_fake_ttl() {
        let mut g1 = make_test_genome();
        let mut g2 = make_test_genome();
        g2.fake_ttl = Some(128);

        assert_ne!(
            GenomeSignature::compute(&g1),
            GenomeSignature::compute(&g2)
        );
    }

    #[test]
    fn test_signature_depends_on_extra_args() {
        let mut g1 = make_test_genome();
        let mut g2 = make_test_genome();
        g2.extra_args.push("--custom-arg".into());

        assert_ne!(
            GenomeSignature::compute(&g1),
            GenomeSignature::compute(&g2)
        );
    }

    fn make_test_genome() -> StrategyGenome {
        let mut g = StrategyGenome::new(DpiEngineType::Zapret, "Test".into());
        g.filter_tcp = "443".into();
        g.filter_udp = "443".into();
        g.desync_mode = "split".into();
        g.fake_ttl = Some(64);
        g.auto_ttl = true;
        g.repeat_count = Some(3);
        g.disorder_pos = Some("3".into());
        g
    }
}
