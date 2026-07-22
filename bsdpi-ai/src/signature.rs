//! GenomeSignature — SHA256 хеш генома для дедупликации стратегий.
//!
//! Два генома с одинаковой сигнатурой считаются эквивалентными.
//! Сигнатура вычисляется по всем параметрам DPI bypass (без метаданных).
//!
//! ## C# оригинал
//! `BSDPI.AI/Models/GenomeSignature.cs`

use crate::genome::StrategyGenome;
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Внутренняя структура для сериализации — только параметры, влияющие на сигнатуру.
#[derive(Serialize)]
struct SignaturePayload<'a> {
    engine_type: &'a str,
    filter_tcp: &'a str,
    filter_udp: &'a str,
    desync_mode: &'a str,
    split_pos: Option<i32>,
    split_pos_semantic: Option<&'a str>,
    disorder_pos: Option<&'a str>,
    fake_pos: Option<&'a str>,
    oob_pos: Option<&'a str>,
    disoob_pos: Option<&'a str>,
    tlsrec_pos: Option<&'a str>,
    fake_ttl: Option<i32>,
    auto_ttl: bool,
    md5sig: Option<bool>,
    fake_tls_mod: Option<&'a str>,
    fake_sni: Option<&'a str>,
    fake_data: Option<&'a str>,
    mod_http: Option<&'a str>,
    tlsminor: Option<i32>,
    hosts: Option<&'a str>,
    hostlist: Option<&'a str>,
    repeat_count: Option<i32>,
    cache_ttl: Option<i32>,
    auto: Option<&'a str>,
    timeout: Option<i32>,
    auto_mode: Option<i32>,
    desync_any_protocol: Option<&'a str>,
    desync_fooling: Option<&'a str>,
    fake_resend: Option<&'a str>,
    warp_config: Option<&'a str>,
    mtu: Option<i32>,
    gool_enabled: bool,
    psiphon_enabled: bool,
    psiphon_country: Option<&'a str>,
    scan_enabled: bool,
    reserved: Option<&'a str>,
    extra: String,
}

/// Вычисляет SHA256 сигнатуру генома.
///
/// Сигнатура не зависит от:
/// - id, parent_ids, generation, origin
/// - display_name, bat_file_name, source_bat_path
/// - created_at, orchestrator_enabled
/// - last_verification_score, last_verified_at
pub fn compute(genome: &StrategyGenome) -> String {
    let payload = SignaturePayload {
        engine_type: genome.engine_type.as_str(),
        filter_tcp: &genome.filter_tcp,
        filter_udp: &genome.filter_udp,
        desync_mode: &genome.desync_mode,
        split_pos: genome.split_pos,
        split_pos_semantic: genome.split_pos_semantic.as_deref(),
        disorder_pos: genome.disorder_pos.as_deref(),
        fake_pos: genome.fake_pos.as_deref(),
        oob_pos: genome.oob_pos.as_deref(),
        disoob_pos: genome.disoob_pos.as_deref(),
        tlsrec_pos: genome.tlsrec_pos.as_deref(),
        fake_ttl: genome.fake_ttl,
        auto_ttl: genome.auto_ttl,
        md5sig: genome.md5sig,
        fake_tls_mod: genome.fake_tls_mod.as_deref(),
        fake_sni: genome.fake_sni.as_deref(),
        fake_data: genome.fake_data.as_deref(),
        mod_http: genome.mod_http.as_deref(),
        tlsminor: genome.tlsminor,
        hosts: genome.hosts.as_deref(),
        hostlist: genome.hostlist.as_deref(),
        repeat_count: genome.repeat_count,
        cache_ttl: genome.cache_ttl,
        auto: genome.auto.as_deref(),
        timeout: genome.timeout,
        auto_mode: genome.auto_mode,
        desync_any_protocol: genome.desync_any_protocol.as_deref(),
        desync_fooling: genome.desync_fooling.as_deref(),
        fake_resend: genome.fake_resend.as_deref(),
        warp_config: genome.warp_config.as_deref(),
        mtu: genome.mtu,
        gool_enabled: genome.gool_enabled,
        psiphon_enabled: genome.psiphon_enabled,
        psiphon_country: genome.psiphon_country.as_deref(),
        scan_enabled: genome.scan_enabled,
        reserved: genome.reserved.as_deref(),
        extra: genome.extra_args.join("\u{1f}"),
    };

    let json = serde_json::to_string(&payload).unwrap_or_default();
    let hash = Sha256::digest(json.as_bytes());
    hex::encode(hash)
}

/// Вычислить сигнатуру для набора геномов (все уникальные).
pub fn compute_set(genomes: &[StrategyGenome]) -> Vec<String> {
    let mut sigs: Vec<String> = genomes.iter().map(compute).collect();
    sigs.sort();
    sigs.dedup();
    sigs
}

/// Проверить, есть ли геном с такой же сигнатурой в наборе.
pub fn exists_in(genome: &StrategyGenome, pool: &[StrategyGenome]) -> bool {
    let sig = compute(genome);
    pool.iter().any(|g| compute(g) == sig)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::StrategyGenome;

    #[test]
    fn test_signature_is_deterministic() {
        let a = StrategyGenome::default_zapret();
        let b = StrategyGenome::default_zapret();
        assert_eq!(compute(&a), compute(&b));
    }

    #[test]
    fn test_signature_differs_for_different_params() {
        let mut a = StrategyGenome::default_zapret();
        let mut b = StrategyGenome::default_zapret();
        b.desync_mode = "fake".into();
        assert_ne!(compute(&a), compute(&b));
    }

    #[test]
    fn test_signature_ignores_metadata() {
        let mut a = StrategyGenome::default_zapret();
        let mut b = StrategyGenome::default_zapret();
        b.display_name = "different-name".into();
        b.id = uuid::Uuid::new_v4();
        assert_eq!(compute(&a), compute(&b));
    }

    #[test]
    fn test_signature_ignores_generation() {
        let mut a = StrategyGenome::default_zapret();
        let mut b = StrategyGenome::default_zapret();
        b.generation = 42;
        assert_eq!(compute(&a), compute(&b));
    }

    #[test]
    fn test_signature_uses_extra_args() {
        let mut a = StrategyGenome::default_zapret();
        let mut b = StrategyGenome::default_zapret();
        b.extra_args.push("--new".into());
        assert_ne!(compute(&a), compute(&b));
    }

    #[test]
    fn test_compute_set_deduplicates() {
        let a = StrategyGenome::default_zapret();
        let b = StrategyGenome::default_zapret();
        let c = StrategyGenome::default_byedpi();
        let sigs = compute_set(&[a, b, c]);
        assert_eq!(sigs.len(), 2); // zapret dupes, byedpi diff
    }

    #[test]
    fn test_exists_in() {
        let a = StrategyGenome::default_zapret();
        let b = StrategyGenome::default_zapret();
        let c = StrategyGenome::default_byedpi();
        assert!(exists_in(&a, &[b, c.clone()]));
        assert!(!exists_in(&c, &[StrategyGenome::default_zapret()]));
    }
}
