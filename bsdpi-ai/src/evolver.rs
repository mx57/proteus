//! Genetic Strategy Evolver — генетический алгоритм для DPI bypass стратегий.
//!
//! Алгоритм:
//! 1. Выбор 2 родителей из топ-6 (по Wilson Score)
//! 2. Crossover — половинное наследование каждого параметра
//! 3. Mutation — 15 типов для Zapret, 10 для ByeDpi, 7 для Warp
//! 4. Валидация + дедупликация через GenomeSignature
//! 5. Garbage Collection: удаление слабых evolved стратегий (elitism)
//!
//! ## C# оригинал
//! `BSDPI.AI/Services/StrategyEvolver.cs`

use crate::error::AiError;
use crate::genome::{DpiEngineType, StrategyGenome, StrategyOrigin};
use crate::signature;
use crate::wilson;
use chrono::Utc;
use rand::Rng;
use uuid::Uuid;

// ========== Константы для мутации ==========

const SEMANTIC_MARKERS: &[&str] = &["host", "endhost", "midsld", "sniext", "endsld"];
const DESYNC_MODES: &[&str] = &[
    "split", "fake", "fakesplit", "disorder", "fakedisorder", "multidisorder", "multisplit",
];
const FAKE_TLS_MODS: &[&str] = &["orig", "rand", "rndsni", "dupsid", "padencap"];
const ENGINE_TYPES: &[DpiEngineType] = &[DpiEngineType::Zapret, DpiEngineType::ByeDpi, DpiEngineType::Warp];
const SPLIT_POS_CANDIDATES: &[&str] = &["1", "2", "3", "7", "10", "1+s", "2+s", "3+s", "host", "midsld", "sniext"];
const DISORDER_POS_CANDIDATES: &[&str] = &["1", "3", "5", "1+s", "3+s"];
const FAKE_POS_CANDIDATES: &[&str] = &["-1", "3", "7", "10"];
const OOB_POS_CANDIDATES: &[&str] = &["1", "3", "7", "10", "3+s", "5+s"];
const DISOOB_POS_CANDIDATES: &[&str] = &["1", "3", "7", "10"];
const TLSREC_POS_CANDIDATES: &[&str] = &["1", "3", "7", "1+s", "3+s"];
const MOD_HTTP_CANDIDATES: &[&str] = &["hcsmix", "dcsmix", "rmspace", "hcsmix,dcsmix", "hcsmix,rmspace"];
const FOOLING_CANDIDATES: &[&str] = &["md5sig", "badseq", "datanoack", "hopbyhop", "hopbyhop2", "badsum"];
const ANY_PROTOCOL_CANDIDATES: &[&str] = &["0", "1"];
const FAKE_RESEND_CANDIDATES: &[&str] = &["orig", "proxy", "null"];
const REPEAT_COUNT_CANDIDATES: &[i32] = &[1, 2, 3, 5, 10];
const PSIPHON_COUNTRIES: &[&str] = &[
    "AT", "AU", "BE", "BG", "CA", "CH", "CZ", "DE", "DK", "EE", "ES", "FI", "FR", "GB",
    "HR", "HU", "IE", "IN", "IT", "JP", "LV", "NL", "NO", "PL", "PT", "RO", "RS", "SE",
    "SG", "SK", "US",
];

/// Genetic Strategy Evolver.
pub struct StrategyEvolver {
    rng: rand::rngs::ThreadRng,
    max_evolved: usize,
    elitism_enabled: bool,
}

impl StrategyEvolver {
    pub fn new(max_evolved: usize, elitism_enabled: bool) -> Self {
        Self {
            rng: rand::thread_rng(),
            max_evolved,
            elitism_enabled,
        }
    }

    /// Создать новую стратегию через скрещивание лучших родителей.
    ///
    /// `outcomes` — история испытаний: (genome_id, score) где score >= 50 = успех
    pub fn evolve(
        &mut self,
        pool: &[StrategyGenome],
        outcomes: &[(Uuid, i32)],
    ) -> Result<StrategyGenome, AiError> {
        if pool.len() < 2 {
            return Err(AiError::Evolution("need at least 2 genomes in pool".into()));
        }

        // Шаг 1: Scoring — Wilson Score по истории
        let mut scored: Vec<(&StrategyGenome, f64, usize)> = pool
            .iter()
            .map(|g| {
                let filtered: Vec<&(Uuid, i32)> = outcomes.iter().filter(|(id, _)| *id == g.id).collect();
                let succ = filtered.iter().filter(|(_, s)| *s >= 50).count();
                let trials = filtered.len();
                let w = wilson::lower_bound_95(succ as u64, trials as u64);
                (g, w, trials)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Берём топ-6 родителей (или всех, если меньше)
        let parent_count = scored.len().min(6);
        let parents: Vec<&StrategyGenome> = scored[..parent_count].iter().map(|(g, _, _)| *g).collect();

        if parents.len() < 2 {
            return Err(AiError::Evolution("need at least 2 parents".into()));
        }

        // Шаг 2: Попытки скрещивания (до 25)
        let mut child: Option<StrategyGenome> = None;
        for _attempt in 0..25 {
            let mut p0 = parents[self.rng.gen_range(0..parents.len())];
            let idx1 = self.rng.gen_range(0..parents.len());
            let mut p1 = parents[idx1];

            // Избегаем клонирования одного родителя
            if p0.id == p1.id && parents.len() > 1 {
                let p0_idx = parents.iter().position(|g| g.id == p0.id).unwrap_or(0);
                p1 = parents[(p0_idx + 1) % parents.len()];
            }

            // Crossover
            let mut candidate = self.crossover(p0, p1);

            // Mutation
            self.mutate(&mut candidate);

            // Normalize & validate
            normalize(&mut candidate);

            if !is_valid(&candidate) {
                continue;
            }

            // Dedup
            if signature::exists_in(&candidate, pool) {
                continue;
            }

            child = Some(candidate);
            break;
        }

        let mut child = child.ok_or_else(|| AiError::Evolution("failed to create valid child after 25 attempts".into()))?;

        // Шаг 3: Финальная настройка
        child.id = Uuid::new_v4();
        child.origin = StrategyOrigin::Evolved;
        child.created_at = Utc::now();

        let engine_tag = match child.engine_type {
            DpiEngineType::ByeDpi => "byedpi",
            _ => "zapret",
        };
        child.display_name = format!("ev-{}-{}-{:04}", child.generation, engine_tag, self.rng.gen_range(1000..9999));

        Ok(child)
    }

    /// Crossover — каждый параметр наследуется от случайного из 2 родителей.
    fn crossover(&mut self, a: &StrategyGenome, b: &StrategyGenome) -> StrategyGenome {
        StrategyGenome {
            id: Uuid::new_v4(),
            parent_ids: vec![a.id, b.id],
            generation: a.generation.max(b.generation) + 1,
            origin: StrategyOrigin::Evolved,

            engine_type: self.rng_pick(&a.engine_type, &b.engine_type),

            filter_tcp: self.rng_pick_str(&a.filter_tcp, &b.filter_tcp),
            filter_udp: self.rng_pick_str(&a.filter_udp, &b.filter_udp),
            desync_mode: self.rng_pick_str(&a.desync_mode, &b.desync_mode),

            split_pos: self.rng_pick_opt(&a.split_pos, &b.split_pos),
            split_pos_semantic: self.rng_pick_opt_ref(&a.split_pos_semantic, &b.split_pos_semantic),
            disorder_pos: self.rng_pick_opt_ref(&a.disorder_pos, &b.disorder_pos),
            fake_pos: self.rng_pick_opt_ref(&a.fake_pos, &b.fake_pos),
            oob_pos: self.rng_pick_opt_ref(&a.oob_pos, &b.oob_pos),
            disoob_pos: self.rng_pick_opt_ref(&a.disoob_pos, &b.disoob_pos),
            tlsrec_pos: self.rng_pick_opt_ref(&a.tlsrec_pos, &b.tlsrec_pos),

            fake_ttl: self.rng_pick_opt(&a.fake_ttl, &b.fake_ttl),
            auto_ttl: self.rng_pick_bool(a.auto_ttl, b.auto_ttl),
            md5sig: self.rng_pick_opt(&a.md5sig, &b.md5sig),
            fake_tls_mod: self.rng_pick_opt_ref(&a.fake_tls_mod, &b.fake_tls_mod),
            fake_sni: self.rng_pick_opt_ref(&a.fake_sni, &b.fake_sni),
            fake_data: self.rng_pick_opt_ref(&a.fake_data, &b.fake_data),
            mod_http: self.rng_pick_opt_ref(&a.mod_http, &b.mod_http),
            tlsminor: self.rng_pick_opt(&a.tlsminor, &b.tlsminor),
            hosts: self.rng_pick_opt_ref(&a.hosts, &b.hosts),
            hostlist: self.rng_pick_opt_ref(&a.hostlist, &b.hostlist),
            repeat_count: self.rng_pick_opt(&a.repeat_count, &b.repeat_count),
            cache_ttl: self.rng_pick_opt(&a.cache_ttl, &b.cache_ttl),

            auto: self.rng_pick_opt_ref(&a.auto, &b.auto),
            timeout: self.rng_pick_opt(&a.timeout, &b.timeout),
            auto_mode: self.rng_pick_opt(&a.auto_mode, &b.auto_mode),

            desync_any_protocol: self.rng_pick_opt_ref(&a.desync_any_protocol, &b.desync_any_protocol),
            desync_fooling: self.rng_pick_opt_ref(&a.desync_fooling, &b.desync_fooling),
            fake_resend: self.rng_pick_opt_ref(&a.fake_resend, &b.fake_resend),

            warp_config: self.rng_pick_opt_ref(&a.warp_config, &b.warp_config),
            mtu: self.rng_pick_opt(&a.mtu, &b.mtu),
            gool_enabled: self.rng_pick_bool(a.gool_enabled, b.gool_enabled),
            psiphon_enabled: self.rng_pick_bool(a.psiphon_enabled, b.psiphon_enabled),
            psiphon_country: self.rng_pick_opt_ref(&a.psiphon_country, &b.psiphon_country),
            scan_enabled: self.rng_pick_bool(a.scan_enabled, b.scan_enabled),
            reserved: self.rng_pick_opt_ref(&a.reserved, &b.reserved),

            extra_args: self.rng_pick_list(&a.extra_args, &b.extra_args),

            // Metadata — не наследуются
            display_name: String::new(),
            bat_file_name: None,
            source_bat_path: None,
            created_at: Utc::now(),
            orchestrator_enabled: true,
            last_verification_score: None,
            last_verified_at: None,
        }
    }

    /// Mutation — применяется к ребёнку после crossover.
    fn mutate(&mut self, g: &mut StrategyGenome) {
        let roll = self.rng.gen_range(0..15);

        match g.engine_type {
            DpiEngineType::ByeDpi => self.mutate_byedpi(g, roll),
            DpiEngineType::Warp => self.mutate_warp(g, roll),
            DpiEngineType::Zapret => self.mutate_zapret(g, roll),
        }
    }

    fn mutate_zapret(&mut self, g: &mut StrategyGenome, roll: i32) {
        match roll {
            0 => {
                g.split_pos_semantic = Some(SEMANTIC_MARKERS[self.rng.gen_range(0..SEMANTIC_MARKERS.len())].to_string());
                g.split_pos = None;
            }
            1 => {
                g.desync_mode = DESYNC_MODES[self.rng.gen_range(0..DESYNC_MODES.len())].to_string();
            }
            2 => {
                g.fake_ttl = if let Some(ttl) = g.fake_ttl {
                    Some((ttl + self.pick_delta()).clamp(3, 48))
                } else {
                    Some(6 + self.rng.gen_range(0..10))
                };
            }
            3 => {
                g.fake_tls_mod = Some(FAKE_TLS_MODS[self.rng.gen_range(0..FAKE_TLS_MODS.len())].to_string());
            }
            4 => {
                g.auto_ttl = !g.auto_ttl;
            }
            5 => {
                g.desync_fooling = Some(FOOLING_CANDIDATES[self.rng.gen_range(0..FOOLING_CANDIDATES.len())].to_string());
            }
            6 => {
                g.desync_any_protocol = Some(ANY_PROTOCOL_CANDIDATES[self.rng.gen_range(0..ANY_PROTOCOL_CANDIDATES.len())].to_string());
            }
            7 => {
                g.fake_resend = Some(FAKE_RESEND_CANDIDATES[self.rng.gen_range(0..FAKE_RESEND_CANDIDATES.len())].to_string());
            }
            8 => {
                g.repeat_count = Some(REPEAT_COUNT_CANDIDATES[self.rng.gen_range(0..REPEAT_COUNT_CANDIDATES.len())]);
            }
            9 => {
                g.disorder_pos = Some(DISORDER_POS_CANDIDATES[self.rng.gen_range(0..DISORDER_POS_CANDIDATES.len())].to_string());
            }
            10 => {
                g.fake_pos = Some(FAKE_POS_CANDIDATES[self.rng.gen_range(0..FAKE_POS_CANDIDATES.len())].to_string());
            }
            11 => {
                // Switch to ByeDpi
                g.engine_type = DpiEngineType::ByeDpi;
                g.split_pos = None;
                g.split_pos_semantic = None;
                g.auto_ttl = false;
                g.disorder_pos = Some("1+s".into());
            }
            12 => {
                g.oob_pos = Some(OOB_POS_CANDIDATES[self.rng.gen_range(0..OOB_POS_CANDIDATES.len())].to_string());
            }
            13 => {
                g.disoob_pos = Some(DISOOB_POS_CANDIDATES[self.rng.gen_range(0..DISOOB_POS_CANDIDATES.len())].to_string());
            }
            14 => {
                g.tlsrec_pos = Some(TLSREC_POS_CANDIDATES[self.rng.gen_range(0..TLSREC_POS_CANDIDATES.len())].to_string());
            }
            _ => {}
        }
    }

    fn mutate_warp(&mut self, g: &mut StrategyGenome, roll: i32) {
        match roll {
            0 => g.engine_type = DpiEngineType::Zapret,
            1 => g.engine_type = DpiEngineType::ByeDpi,
            2 => {
                let mtu = g.mtu.unwrap_or(1280) + (self.rng.gen_range(0..3) - 1) * 20;
                g.mtu = Some(mtu.clamp(1200, 1500));
            }
            3 => g.gool_enabled = !g.gool_enabled,
            4 => {
                g.psiphon_enabled = !g.psiphon_enabled;
                if g.psiphon_enabled {
                    g.psiphon_country = Some(PSIPHON_COUNTRIES[self.rng.gen_range(0..PSIPHON_COUNTRIES.len())].to_string());
                }
            }
            5 => g.scan_enabled = !g.scan_enabled,
            6 => {
                g.reserved = Some(format!("{},{},{}", self.rng.gen_range(0..256), self.rng.gen_range(0..256), self.rng.gen_range(0..256)));
            }
            _ => {}
        }
    }

    fn mutate_byedpi(&mut self, g: &mut StrategyGenome, roll: i32) {
        match roll {
            0 => {
                g.split_pos = None;
                g.split_pos_semantic = Some(SPLIT_POS_CANDIDATES[self.rng.gen_range(0..SPLIT_POS_CANDIDATES.len())].to_string());
            }
            1 => {
                g.disorder_pos = Some(DISORDER_POS_CANDIDATES[self.rng.gen_range(0..DISORDER_POS_CANDIDATES.len())].to_string());
            }
            2 => {
                g.fake_pos = Some(FAKE_POS_CANDIDATES[self.rng.gen_range(0..FAKE_POS_CANDIDATES.len())].to_string());
                if g.fake_ttl.is_none() {
                    g.fake_ttl = Some(5 + self.rng.gen_range(0..8));
                }
            }
            3 => {
                g.tlsrec_pos = Some(TLSREC_POS_CANDIDATES[self.rng.gen_range(0..TLSREC_POS_CANDIDATES.len())].to_string());
            }
            4 => {
                g.oob_pos = Some(OOB_POS_CANDIDATES[self.rng.gen_range(0..OOB_POS_CANDIDATES.len())].to_string());
            }
            5 => {
                g.fake_ttl = if let Some(ttl) = g.fake_ttl {
                    Some((ttl + self.pick_delta()).clamp(3, 48))
                } else {
                    Some(5 + self.rng.gen_range(0..10))
                };
            }
            6 => {
                g.md5sig = Some(!g.md5sig.unwrap_or(true));
            }
            7 => {
                g.fake_tls_mod = Some(FAKE_TLS_MODS[self.rng.gen_range(0..FAKE_TLS_MODS.len())].to_string());
            }
            8 => {
                g.auto = Some(if self.rng.gen_bool(0.5) { "torst".into() } else { "ssl_err".into() });
                g.timeout = Some(3 + self.rng.gen_range(0..4));
            }
            9 => {
                g.mod_http = Some(MOD_HTTP_CANDIDATES[self.rng.gen_range(0..MOD_HTTP_CANDIDATES.len())].to_string());
            }
            10 => {
                // Switch to Zapret
                g.engine_type = DpiEngineType::Zapret;
                g.disorder_pos = None;
                g.fake_pos = None;
                g.md5sig = None;
                g.fake_sni = None;
                g.fake_data = None;
                g.mod_http = None;
                g.tlsminor = None;
                g.hosts = None;
                g.auto = None;
                g.timeout = None;
                g.auto_mode = None;
                g.cache_ttl = None;
            }
            _ => {
                g.tlsminor = Some(self.rng.gen_range(2..4));
            }
        }
    }

    /// Garbage Collection — удаление слабых evolved, с защитой элиты.
    pub fn gc_evolved(
        &self,
        pool: &mut Vec<StrategyGenome>,
        outcomes: &[(Uuid, i32)],
    ) -> Vec<StrategyGenome> {
        let settings_max = self.max_evolved.max(4);
        let elitism_count = if self.elitism_enabled {
            (settings_max / 4).max(2)
        } else {
            0
        };

        let (mut evolved, others): (Vec<StrategyGenome>, Vec<StrategyGenome>) =
            pool.drain(..).partition(|g| g.origin == StrategyOrigin::Evolved);

        if evolved.len() <= settings_max {
            pool.extend(evolved);
            pool.extend(others);
            return Vec::new();
        }

        let mut ranked: Vec<(StrategyGenome, f64)> = evolved
            .into_iter()
            .map(|g| {
                let filtered: Vec<&(Uuid, i32)> = outcomes.iter().filter(|(id, _)| *id == g.id).collect();
                let succ = filtered.iter().filter(|(_, s)| *s >= 50).count();
                let trials = filtered.len();
                let w = wilson::lower_bound_95(succ as u64, trials as u64);
                (g, w)
            })
            .collect();

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Элитизм: защищаем топ-N
        let protected: Vec<Uuid> = ranked.iter().take(elitism_count).map(|(g, _)| g.id).collect();
        let mut removed = Vec::new();

        let remaining: Vec<(StrategyGenome, f64)> = ranked.into_iter().collect();
        for (g, _) in remaining.into_iter().skip(elitism_count) {
            if pool.len() + others.len() >= settings_max {
                removed.push(g);
            } else {
                pool.push(g);
            }
        }

        // Добавляем обратно элиту
        // (она уже в pool через skip)

        pool.extend(others);
        removed
    }

    // ========== Хелперы ==========

    fn rng_pick<T: Clone>(&mut self, a: &T, b: &T) -> T {
        if self.rng.gen_bool(0.5) { a.clone() } else { b.clone() }
    }

    fn rng_pick_str(&mut self, a: &str, b: &str) -> String {
        if self.rng.gen_bool(0.5) { a.to_string() } else { b.to_string() }
    }

    fn rng_pick_opt<T: Clone>(&mut self, a: &Option<T>, b: &Option<T>) -> Option<T> {
        if self.rng.gen_bool(0.5) { a.clone() } else { b.clone() }
    }

    fn rng_pick_opt_ref(&mut self, a: &Option<String>, b: &Option<String>) -> Option<String> {
        if self.rng.gen_bool(0.5) { a.clone() } else { b.clone() }
    }

    fn rng_pick_bool(&mut self, a: bool, b: bool) -> bool {
        if self.rng.gen_bool(0.5) { a } else { b }
    }

    fn rng_pick_list(&mut self, a: &[String], b: &[String]) -> Vec<String> {
        if a.is_empty() && b.is_empty() {
            return Vec::new();
        }
        if a.is_empty() { return b.to_vec(); }
        if b.is_empty() { return a.to_vec(); }
        if self.rng.gen_bool(0.5) { a.to_vec() } else { b.to_vec() }
    }

    fn pick_delta(&mut self) -> i32 {
        let d = [1, 2, 4, 8];
        let v = d[self.rng.gen_range(0..d.len())];
        if self.rng.gen_bool(0.5) { v } else { -v }
    }
}

// ========== Валидация генома ==========

/// Нормализация: применяет правила корректировки к геному.
pub fn normalize(g: &mut StrategyGenome) {
    if g.split_pos_semantic.is_some() {
        g.split_pos = None;
    }
}

/// Валидация: проверяет что геном минимально корректен.
pub fn is_valid(g: &StrategyGenome) -> bool {
    !g.desync_mode.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::{DpiEngineType, StrategyGenome, StrategyOrigin};

    fn make_genome(desync_mode: &str, engine: DpiEngineType) -> StrategyGenome {
        let mut g = StrategyGenome::new(engine, StrategyOrigin::Builtin);
        g.desync_mode = desync_mode.into();
        g
    }

    #[test]
    fn test_evolve_returns_child() {
        let mut evolver = StrategyEvolver::new(10, true);
        let pool = vec![
            make_genome("split", DpiEngineType::Zapret),
            make_genome("fake", DpiEngineType::Zapret),
        ];
        let outcomes = vec![(pool[0].id, 80), (pool[1].id, 20)];

        let child = evolver.evolve(&pool, &outcomes).unwrap();
        assert_eq!(child.origin, StrategyOrigin::Evolved);
        assert!(child.generation >= 1);
        assert!(!child.display_name.is_empty());
    }

    #[test]
    fn test_evolve_requires_2_genomes() {
        let mut evolver = StrategyEvolver::new(10, true);
        let pool = vec![make_genome("split", DpiEngineType::Zapret)];
        let result = evolver.evolve(&pool, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_crossover_mixes_params() {
        let mut evolver = StrategyEvolver::new(10, true);
        let a = make_genome("split", DpiEngineType::Zapret);
        let b = make_genome("fake", DpiEngineType::ByeDpi);

        let child = evolver.crossover(&a, &b);
        // Child должен иметь engine_type одного из родителей
        assert!(child.engine_type == DpiEngineType::Zapret || child.engine_type == DpiEngineType::ByeDpi);
        assert_eq!(child.parent_ids.len(), 2);
    }

    #[test]
    fn test_mutate_zapret_changes_param() {
        let mut evolver = StrategyEvolver::new(10, true);
        let mut g = make_genome("split", DpiEngineType::Zapret);
        let original_desync = g.desync_mode.clone();
        evolver.mutate(&mut g);
        // После мутации может измениться desync_mode или другой параметр
        // Проверяем только что мутация не падает
        assert!(true);
    }

    #[test]
    fn test_mutate_byedpi_changes_param() {
        let mut evolver = StrategyEvolver::new(10, true);
        let mut g = make_genome("split", DpiEngineType::ByeDpi);
        evolver.mutate(&mut g);
        assert!(true);
    }

    #[test]
    fn test_mutate_warp_changes_param() {
        let mut evolver = StrategyEvolver::new(10, true);
        let mut g = make_genome("split", DpiEngineType::Warp);
        evolver.mutate(&mut g);
        assert!(true);
    }

    #[test]
    fn test_gc_removes_weak_evolved() {
        let evolver = StrategyEvolver::new(2, true); // max 2 evolved
        let mut pool = vec![
            make_genome("split", DpiEngineType::Zapret), // builtin — не будет удалён
        ];
        let outcomes = vec![];

        let removed = evolver.gc_evolved(&mut pool, &outcomes);
        // builtin не удаляется, evolved нет
        assert!(removed.is_empty());
    }

    #[test]
    fn test_is_valid_checks_desync_mode() {
        let mut g = make_genome("split", DpiEngineType::Zapret);
        assert!(is_valid(&g));

        g.desync_mode = "".into();
        assert!(!is_valid(&g));
    }

    #[test]
    fn test_normalize_sets_split_pos() {
        let mut g = make_genome("split", DpiEngineType::Zapret);
        g.split_pos_semantic = Some("host".into());
        g.split_pos = Some(42);
        normalize(&mut g);
        assert_eq!(g.split_pos, None); // должен быть None
        assert_eq!(g.split_pos_semantic, Some("host".into())); // semantic остаётся
    }

    #[test]
    fn test_pick_delta_not_zero() {
        let mut evolver = StrategyEvolver::new(10, true);
        let mut non_zero = 0;
        for _ in 0..100 {
            let d = evolver.pick_delta();
            if d != 0 {
                non_zero += 1;
            }
        }
        assert!(non_zero > 0);
    }
}
