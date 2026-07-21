//! AiStrategyRegistry — персистентное хранилище стратегий.
//!
//! Порт C# `BSDPI.AI/Services/AiStrategyRegistry.cs`:
//! - In-memory модель с JSON-сериализацией на диск
//! - Thread-safe через Mutex
//! - lookup-словари для O(1) доступа
//! - Поддержка экпорта/импорта обученных стратегий

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::genome::{StrategyGenome, StrategyOrigin};

// ─── Data models ───

/// Состояние bandit для одной стратегии на одной сети.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BanditStateEntry {
    pub genome_id: String,
    pub network_hash: String,
    pub alpha: f64,
    pub beta: f64,
    pub avg_latency: f64,
}

impl BanditStateEntry {
    pub fn new(genome_id: String, network_hash: String) -> Self {
        Self {
            genome_id,
            network_hash,
            alpha: 1.0,
            beta: 1.0,
            avg_latency: 1000.0,
        }
    }

    pub fn pulls(&self) -> f64 {
        (self.alpha + self.beta - 2.0).max(0.0)
    }
}

/// Persisted model — полный JSON снимок состояния.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiStrategyPersistedModel {
    pub genomes: Vec<StrategyGenome>,
    pub bandit: Vec<BanditStateEntry>,
    pub generation_counter: u32,
    pub seen_network_hashes: Vec<String>,
}

impl Default for AiStrategyPersistedModel {
    fn default() -> Self {
        Self {
            genomes: Vec::new(),
            bandit: Vec::new(),
            generation_counter: 0,
            seen_network_hashes: Vec::new(),
        }
    }
}

/// Статус записи стратегии.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordStatus {
    Active,
    Archived,
    Banned,
}

/// Реестр AI-стратегий.
pub struct AiStrategyRegistry {
    path: String,
    inner: Arc<Mutex<RegistryInner>>,
}

struct RegistryInner {
    model: AiStrategyPersistedModel,
    // O(1) lookup's
    bandit_lookup: HashMap<(String, String), BanditStateEntry>,
    genome_lookup: HashMap<String, Vec<BanditStateEntry>>,
    network_lookup: HashMap<String, Vec<BanditStateEntry>>,
    genome_id_lookup: HashMap<String, StrategyGenome>,
}

impl AiStrategyRegistry {
    /// Создаёт реестр с указанием пути для сохранения.
    pub fn new(path: String) -> Self {
        let inner = RegistryInner {
            model: AiStrategyPersistedModel::default(),
            bandit_lookup: HashMap::new(),
            genome_lookup: HashMap::new(),
            network_lookup: HashMap::new(),
            genome_id_lookup: HashMap::new(),
        };

        Self {
            path,
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// Загрузка данных с диска.
    pub fn load(&self) {
        let mut inner = self.inner.lock().unwrap();
        let path = Path::new(&self.path);

        if !path.exists() {
            inner.model = AiStrategyPersistedModel::default();
            return;
        }

        match std::fs::read_to_string(path) {
            Ok(json) => {
                match serde_json::from_str::<AiStrategyPersistedModel>(&json) {
                    Ok(model) => {
                        inner.model = model;
                    }
                    Err(_) => {
                        inner.model = AiStrategyPersistedModel::default();
                    }
                }
            }
            Err(_) => {
                inner.model = AiStrategyPersistedModel::default();
            }
        }

        inner.rebuild_lookups();
    }

    /// Сохранение данных на диск.
    pub fn save(&self) {
        let inner = self.inner.lock().unwrap();
        let json = serde_json::to_string_pretty(&inner.model).unwrap_or_default();

        if let Some(dir) = Path::new(&self.path).parent() {
            let _ = std::fs::create_dir_all(dir);
        }

        // Atomic write: сначала во временный файл, потом переименовать
        let tmp_path = format!("{}.{}.tmp", self.path, uuid::Uuid::new_v4());
        if std::fs::write(&tmp_path, &json).is_ok() {
            let _ = std::fs::rename(&tmp_path, &self.path);
        }
    }

    /// Получить все геномы.
    pub fn get_genomes(&self) -> Vec<StrategyGenome> {
        let inner = self.inner.lock().unwrap();
        inner.model.genomes.clone()
    }

    /// Получить активные геномы (OrchestratorEnabled = true).
    pub fn get_active_genomes(&self) -> Vec<StrategyGenome> {
        let inner = self.inner.lock().unwrap();
        inner.model.genomes.iter()
            .filter(|g| g.orchestrator_enabled)
            .cloned()
            .collect()
    }

    /// Включить/выключить оркестратор для стратегии.
    pub fn set_orchestrator_enabled(&self, id: &str, enabled: bool) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(g) = inner.model.genomes.iter_mut().find(|g| g.id == id) {
            g.orchestrator_enabled = enabled;
        }
        drop(inner);
        self.save();
    }

    /// Получить геном по ID.
    pub fn get_by_id(&self, id: &str) -> Option<StrategyGenome> {
        let inner = self.inner.lock().unwrap();
        inner.genome_id_lookup.get(id).cloned()
    }

    /// Вставить или обновить геном.
    pub fn upsert(&self, genome: StrategyGenome) {
        let mut inner = self.inner.lock().unwrap();
        let id = genome.id.clone();
        if let Some(idx) = inner.model.genomes.iter().position(|g| g.id == id) {
            inner.model.genomes[idx] = genome.clone();
        } else {
            inner.model.genomes.push(genome.clone());
        }
        inner.genome_id_lookup.insert(id, genome);
    }

    /// Удалить геном по ID.
    pub fn remove(&self, id: &str) -> bool {
        let mut inner = self.inner.lock().unwrap();
        let n = inner.model.genomes.iter().position(|g| g.id == id);
        if let Some(idx) = n {
            inner.model.genomes.remove(idx);
            inner.model.bandit.retain(|b| b.genome_id != id);
            inner.genome_id_lookup.remove(id);
            inner.rebuild_lookups();
            true
        } else {
            false
        }
    }

    /// Отметить сеть как "просмотренную".
    pub fn mark_network_seen(&self, hash: &str) {
        let mut inner = self.inner.lock().unwrap();
        if !inner.model.seen_network_hashes.contains(&hash.to_string()) {
            inner.model.seen_network_hashes.push(hash.to_string());
        }
    }

    /// Проверить, просмотрена ли сеть.
    pub fn has_seen_network(&self, hash: &str) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.model.seen_network_hashes.contains(&hash.to_string())
    }

    /// Получить или создать BanditStateEntry для пары (genome, network).
    pub fn get_or_create_bandit(&self, genome_id: &str, network_hash: &str) -> BanditStateEntry {
        let mut inner = self.inner.lock().unwrap();
        let key = (genome_id.to_string(), network_hash.to_string());
        if let Some(entry) = inner.bandit_lookup.get(&key) {
            return entry.clone();
        }

        let entry = BanditStateEntry::new(genome_id.to_string(), network_hash.to_string());
        inner.model.bandit.push(entry.clone());
        inner.add_entry_to_lookups(&entry);
        entry
    }

    /// Записать успех bandit.
    pub fn record_bandit_success(&self, genome_id: &str, network_hash: &str, latency_ms: f64) {
        let mut inner = self.inner.lock().unwrap();
        let key = (genome_id.to_string(), network_hash.to_string());

        // Найти и обновить в model.bandit (источник правды)
        let updated = if let Some(idx) = inner.model.bandit.iter().position(|e| e.genome_id == genome_id && e.network_hash == network_hash) {
            let e = &mut inner.model.bandit[idx];
            e.alpha += 1.0;
            Self::update_latency(e, latency_ms);
            e.clone()
        } else {
            let mut e = BanditStateEntry::new(genome_id.to_string(), network_hash.to_string());
            e.alpha += 1.0;
            e.avg_latency = latency_ms;
            e
        };

        // Перестроить lookups
        inner.model.bandit.retain(|e| !(e.genome_id == genome_id && e.network_hash == network_hash));
        inner.model.bandit.push(updated);
        inner.rebuild_lookups();
    }

    /// Записать неудачу bandit.
    pub fn record_bandit_failure(&self, genome_id: &str, network_hash: &str, latency_ms: f64) {
        let mut inner = self.inner.lock().unwrap();

        let updated = if let Some(idx) = inner.model.bandit.iter().position(|e| e.genome_id == genome_id && e.network_hash == network_hash) {
            let e = &mut inner.model.bandit[idx];
            e.beta += 1.0;
            Self::update_latency(e, latency_ms);
            e.clone()
        } else {
            let mut e = BanditStateEntry::new(genome_id.to_string(), network_hash.to_string());
            e.beta += 1.0;
            e.avg_latency = latency_ms;
            e
        };

        inner.model.bandit.retain(|e| !(e.genome_id == genome_id && e.network_hash == network_hash));
        inner.model.bandit.push(updated);
        inner.rebuild_lookups();
    }

    /// Сумма pull'ов для генома на сети.
    pub fn sum_pulls_for_genome_on_network(&self, genome_id: &str, network_hash: &str) -> f64 {
        let inner = self.inner.lock().unwrap();
        let key = (genome_id.to_string(), network_hash.to_string());
        inner.bandit_lookup.get(&key)
            .map(|e| e.pulls())
            .unwrap_or(0.0)
    }

    /// Агрегированная Beta для генома (по всем сетям).
    pub fn get_aggregated_beta(&self, genome_id: &str) -> (f64, f64) {
        let inner = self.inner.lock().unwrap();
        let mut succ = 0.0;
        let mut fail = 0.0;
        if let Some(entries) = inner.genome_lookup.get(genome_id) {
            for x in entries {
                succ += x.alpha - 1.0;
                fail += x.beta - 1.0;
            }
        }
        (succ + 1.0, fail + 1.0)
    }

    /// Общее количество pull'ов на сети.
    pub fn total_pulls_on_network(&self, network_hash: &str) -> f64 {
        let inner = self.inner.lock().unwrap();
        inner.network_lookup.get(network_hash)
            .map(|entries| entries.iter().map(|e| e.pulls()).sum())
            .unwrap_or(0.0)
            .max(0.0)
    }

    /// Снимок bandit для сети (snapshot).
    pub fn get_bandit_snapshot(&self, network_hash: &str) -> HashMap<String, BanditStateEntry> {
        let inner = self.inner.lock().unwrap();
        inner.network_lookup.get(network_hash)
            .map(|entries| {
                entries.iter().map(|e| (e.genome_id.clone(), e.clone())).collect()
            })
            .unwrap_or_default()
    }

    /// Агрегированная статистика по всем геномам (snapshot).
    pub fn get_aggregated_stats_snapshot(&self) -> HashMap<String, (f64, f64)> {
        let inner = self.inner.lock().unwrap();
        let mut result = HashMap::new();
        for (genome_id, entries) in &inner.genome_lookup {
            let mut succ = 0.0;
            let mut fail = 0.0;
            for x in entries {
                succ += x.alpha - 1.0;
                fail += x.beta - 1.0;
            }
            result.insert(genome_id.clone(), (succ + 1.0, fail + 1.0));
        }
        result
    }

    /// Сбросить все данные.
    pub fn reset_all(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.model = AiStrategyPersistedModel::default();
        inner.rebuild_lookups();
        drop(inner);
        self.save();
    }

    /// Получить "обученные" стратегии (эволюционировавшие и ручные).
    pub fn get_trained_genomes(&self) -> Vec<StrategyGenome> {
        let inner = self.inner.lock().unwrap();
        inner.model.genomes.iter()
            .filter(|g| matches!(g.origin, StrategyOrigin::Evolved | StrategyOrigin::Manual))
            .cloned()
            .collect()
    }

    /// Экспортировать обученные стратегии в JSON.
    pub fn export_strategies(&self, path: &str) {
        let genomes = self.get_trained_genomes();
        let json = serde_json::to_string_pretty(&genomes).unwrap_or_default();
        if let Some(dir) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp_path = format!("{}.{}.tmp", path, uuid::Uuid::new_v4());
        if std::fs::write(&tmp_path, &json).is_ok() {
            let _ = std::fs::rename(&tmp_path, path);
        }
    }

    /// Импортировать стратегии из JSON (порт C# ImportStrategies).
    pub fn import_strategies(&self, path: &str, merge: bool) -> usize {
        let path = std::path::Path::new(path);
        if !path.exists() {
            return 0;
        }

        let json = match std::fs::read_to_string(path) {
            Ok(j) => j,
            Err(_) => return 0,
        };

        let imported: Vec<StrategyGenome> = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(_) => return 0,
        };

        if imported.is_empty() {
            return 0;
        }

        let mut inner = self.inner.lock().unwrap();
        let mut added = 0;

        for mut g in imported {
            if g.origin == StrategyOrigin::Builtin {
                g.origin = StrategyOrigin::Manual;
            }

            let existing = inner.genome_id_lookup.get(&g.id);
            match existing {
                Some(_) if !merge => continue,
                Some(_) => {
                    if let Some(idx) = inner.model.genomes.iter().position(|x| x.id == g.id) {
                        inner.model.genomes[idx] = g.clone();
                    }
                    inner.genome_id_lookup.insert(g.id.clone(), g);
                }
                None => {
                    inner.model.genomes.push(g.clone());
                    inner.genome_id_lookup.insert(g.id.clone(), g);
                    added += 1;
                }
            }
        }

        added
    }

    // ─── Private helpers ───

    fn update_latency(entry: &mut BanditStateEntry, new_lat: f64) {
        if new_lat <= 0.0 { return; }
        const ALPHA: f64 = 0.2;
        if entry.avg_latency >= 999.0 {
            entry.avg_latency = new_lat;
        } else {
            entry.avg_latency = entry.avg_latency * (1.0 - ALPHA) + new_lat * ALPHA;
        }
    }
}

impl RegistryInner {
    fn rebuild_lookups(&mut self) {
        self.bandit_lookup.clear();
        self.genome_lookup.clear();
        self.network_lookup.clear();
        self.genome_id_lookup.clear();

        for g in &self.model.genomes {
            self.genome_id_lookup.insert(g.id.clone(), g.clone());
        }

        let bandit_snapshot: Vec<_> = self.model.bandit.iter().cloned().collect();
        for e in &bandit_snapshot {
            self.add_entry_to_lookups(e);
        }
    }

    fn add_entry_to_lookups(&mut self, e: &BanditStateEntry) {
        let key = (e.genome_id.clone(), e.network_hash.clone());
        self.bandit_lookup.insert(key, e.clone());

        self.genome_lookup
            .entry(e.genome_id.clone())
            .or_default()
            .push(e.clone());

        self.network_lookup
            .entry(e.network_hash.clone())
            .or_default()
            .push(e.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::{DpiEngineType, StrategyGenome};

    fn make_genome(id: &str) -> StrategyGenome {
        let mut g = StrategyGenome::new(DpiEngineType::Zapret, format!("test-{}", id));
        g.id = id.to_string();
        g
    }

    #[test]
    fn test_registry_create() {
        let r = AiStrategyRegistry::new("/tmp/bsdpi-test-registry.json".into());
        assert!(r.get_genomes().is_empty());
    }

    #[test]
    fn test_upsert_and_get() {
        let r = AiStrategyRegistry::new("/tmp/bsdpi-test-upsert.json".into());
        let g = make_genome("g1");
        r.upsert(g.clone());

        let genomes = r.get_genomes();
        assert_eq!(genomes.len(), 1);
        assert_eq!(genomes[0].id, "g1");

        let found = r.get_by_id("g1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "g1");
    }

    #[test]
    fn test_upsert_update() {
        let r = AiStrategyRegistry::new("/tmp/bsdpi-test-update.json".into());
        let mut g = make_genome("g1");
        r.upsert(g.clone());

        g.display_name = "Updated".into();
        r.upsert(g);

        let genomes = r.get_genomes();
        assert_eq!(genomes.len(), 1);
        assert_eq!(genomes[0].display_name, "Updated");
    }

    #[test]
    fn test_remove() {
        let r = AiStrategyRegistry::new("/tmp/bsdpi-test-remove.json".into());
        r.upsert(make_genome("g1"));
        r.upsert(make_genome("g2"));

        assert!(r.remove("g1"));
        assert_eq!(r.get_genomes().len(), 1);
        assert!(!r.remove("nonexistent"));
    }

    #[test]
    fn test_active_genomes() {
        let r = AiStrategyRegistry::new("/tmp/bsdpi-test-active.json".into());
        let mut g1 = make_genome("g1");
        g1.orchestrator_enabled = true;
        let mut g2 = make_genome("g2");
        g2.orchestrator_enabled = false;
        r.upsert(g1);
        r.upsert(g2);

        let active = r.get_active_genomes();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "g1");
    }

    #[test]
    fn test_set_orchestrator_enabled() {
        let r = AiStrategyRegistry::new("/tmp/bsdpi-test-orch.json".into());
        r.upsert(make_genome("g1"));
        r.set_orchestrator_enabled("g1", false);
        let active = r.get_active_genomes();
        assert!(active.is_empty());
    }

    #[test]
    fn test_bandit_operations() {
        let r = AiStrategyRegistry::new("/tmp/bsdpi-test-bandit.json".into());
        r.upsert(make_genome("g1"));

        r.record_bandit_success("g1", "net1", 50.0);
        r.record_bandit_success("g1", "net1", 30.0);
        r.record_bandit_failure("g1", "net1", 100.0);

        let entry = r.get_or_create_bandit("g1", "net1");
        assert_eq!(entry.alpha, 3.0, "2 successes + initial 1");
        assert_eq!(entry.beta, 2.0, "1 failure + initial 1");
    }

    #[test]
    fn test_aggregated_beta() {
        let r = AiStrategyRegistry::new("/tmp/bsdpi-test-agg.json".into());
        r.upsert(make_genome("g1"));

        r.record_bandit_success("g1", "net1", 50.0);
        r.record_bandit_success("g1", "net2", 30.0);
        r.record_bandit_failure("g1", "net1", 100.0);

        let (alpha, beta) = r.get_aggregated_beta("g1");
        assert!((alpha - 3.0).abs() < 0.01, "alpha should be ~3, got {}", alpha);
        assert!((beta - 2.0).abs() < 0.01, "beta should be ~2, got {}", beta);
    }

    #[test]
    fn test_network_tracking() {
        let r = AiStrategyRegistry::new("/tmp/bsdpi-test-net.json".into());
        assert!(!r.has_seen_network("test-net"));
        r.mark_network_seen("test-net");
        assert!(r.has_seen_network("test-net"));
    }

    #[test]
    fn test_bandit_snapshot() {
        let r = AiStrategyRegistry::new("/tmp/bsdpi-test-snap.json".into());
        r.upsert(make_genome("g1"));
        r.upsert(make_genome("g2"));

        r.record_bandit_success("g1", "net1", 50.0);
        r.record_bandit_failure("g2", "net1", 100.0);

        let snap = r.get_bandit_snapshot("net1");
        assert_eq!(snap.len(), 2);
        assert!(snap.contains_key("g1"));
        assert!(snap.contains_key("g2"));
    }

    #[test]
    fn test_aggregated_stats_snapshot() {
        let r = AiStrategyRegistry::new("/tmp/bsdpi-test-agg-snap.json".into());
        r.upsert(make_genome("g1"));
        r.upsert(make_genome("g2"));

        r.record_bandit_success("g1", "net1", 50.0);
        r.record_bandit_failure("g2", "net1", 100.0);

        let stats = r.get_aggregated_stats_snapshot();
        assert!(stats.contains_key("g1"));
        assert!(stats.contains_key("g2"));
    }

    #[test]
    fn test_save_and_load() {
        let path = "/tmp/bsdpi-test-saveload.json";
        let _ = std::fs::remove_file(path);

        {
            let r = AiStrategyRegistry::new(path.into());
            r.upsert(make_genome("g1"));
            r.upsert(make_genome("g2"));
            r.mark_network_seen("test-net");
            r.save();
        }

        {
            let r = AiStrategyRegistry::new(path.into());
            r.load();
            assert_eq!(r.get_genomes().len(), 2);
            assert!(r.has_seen_network("test-net"));
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_trained_genomes() {
        let r = AiStrategyRegistry::new("/tmp/bsdpi-test-trained.json".into());
        let mut g1 = make_genome("g1");
        g1.origin = StrategyOrigin::Builtin;
        let mut g2 = make_genome("g2");
        g2.origin = StrategyOrigin::Evolved;
        let mut g3 = make_genome("g3");
        g3.origin = StrategyOrigin::Manual;

        r.upsert(g1);
        r.upsert(g2);
        r.upsert(g3);

        let trained = r.get_trained_genomes();
        assert_eq!(trained.len(), 2);
        assert!(trained.iter().any(|g| g.id == "g2"));
        assert!(trained.iter().any(|g| g.id == "g3"));
    }

    #[test]
    fn test_empty_load_doesnt_crash() {
        let path = "/tmp/bsdpi-test-nonexistent.json";
        let _ = std::fs::remove_file(path);
        let r = AiStrategyRegistry::new(path.into());
        r.load(); // shouldn't panic
        assert!(r.get_genomes().is_empty());
    }
}
