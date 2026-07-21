//! AiHistoryStore — лог истории проверок AI-оркестратора.
//!
//! Порт C# `BSDPI.AI/Services/AiHistoryStore.cs`:
//! - Append-only JSON-Lines файл
//! - In-memory cache для быстрых чтений
//! - Фильтрация по genome, network, временному окну
//! - Ротация старых записей

use std::sync::{Arc, Mutex};
use std::path::Path;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Результат проверки стратегии.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeOutcome {
    pub genome_id: String,
    pub network_hash: String,
    pub timestamp: DateTime<Utc>,
    pub score: u32,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub process_stable: bool,
    pub failed_target_keys: Vec<String>,
    pub failure_signature: Option<String>,
}

impl ProbeOutcome {
    pub fn new(genome_id: String, network_hash: String) -> Self {
        Self {
            genome_id,
            network_hash,
            timestamp: Utc::now(),
            score: 0,
            success_rate: 0.0,
            avg_latency_ms: 0.0,
            process_stable: true,
            failed_target_keys: Vec::new(),
            failure_signature: None,
        }
    }
}

/// Тип события (для лога).
#[derive(Debug, Clone)]
pub enum WorkEvent {
    StrategySelected(String),
    StrategySucceeded(String),
    StrategyFailed(String),
    EvolutionCompleted,
    FingerprintChanged(String),
}

/// Результат работы.
#[derive(Debug, Clone)]
pub enum WorkResult {
    Success,
    Failure(String),
    Skipped,
}

/// Запись истории.
#[derive(Debug, Clone)]
pub struct HistoryRecord {
    pub id: String,
    pub event: WorkEvent,
    pub result: WorkResult,
    pub timestamp: DateTime<Utc>,
}

impl HistoryRecord {
    pub fn new(event: WorkEvent, result: WorkResult) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            event,
            result,
            timestamp: Utc::now(),
        }
    }
}

/// Хранилище истории проверок.
pub struct AiHistoryStore {
    path: String,
    inner: Arc<Mutex<HistoryInner>>,
}

struct HistoryInner {
    cache: Option<Vec<ProbeOutcome>>,
}

impl AiHistoryStore {
    pub fn new(path: String) -> Self {
        Self {
            path,
            inner: Arc::new(Mutex::new(HistoryInner { cache: None })),
        }
    }

    /// Добавить запись в лог (append).
    pub fn append(&self, outcome: ProbeOutcome) {
        let json = serde_json::to_string(&outcome).unwrap_or_default();
        let mut inner = self.inner.lock().unwrap();

        if let Some(dir) = Path::new(&self.path).parent() {
            let _ = std::fs::create_dir_all(dir);
        }

        // Append to file
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            use std::io::Write;
            let _ = writeln!(file, "{}", json);
        }

        // Update cache
        if let Some(ref mut cache) = inner.cache {
            cache.push(outcome);
        }
    }

    /// Загрузить записи за последний временной интервал.
    pub fn load_recent(&self, window: chrono::Duration) -> Vec<ProbeOutcome> {
        let cutoff = Utc::now() - window;
        self.load_all().into_iter()
            .filter(|o| o.timestamp >= cutoff)
            .collect()
    }

    /// Загрузить записи для конкретной стратегии на конкретной сети.
    pub fn load_for(&self, genome_id: &str, network_hash: &str) -> Vec<ProbeOutcome> {
        self.load_all().into_iter()
            .filter(|o| o.genome_id == genome_id && o.network_hash == network_hash)
            .collect()
    }

    /// Загрузить записи для сети.
    pub fn load_for_network(&self, network_hash: &str) -> Vec<ProbeOutcome> {
        self.load_all().into_iter()
            .filter(|o| o.network_hash == network_hash)
            .collect()
    }

    /// Загрузить все записи (с кэшированием).
    pub fn load_all(&self) -> Vec<ProbeOutcome> {
        let mut inner = self.inner.lock().unwrap();

        if let Some(ref cache) = inner.cache {
            return cache.clone();
        }

        let path = Path::new(&self.path);
        if !path.exists() {
            inner.cache = Some(Vec::new());
            return Vec::new();
        }

        let mut list = Vec::new();
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() { continue; }
                if let Ok(o) = serde_json::from_str::<ProbeOutcome>(line) {
                    list.push(o);
                }
            }
        }

        inner.cache = Some(list.clone());
        list
    }

    /// Ротация старых записей (удаление старше N дней).
    pub fn rotate_old_entries(&self, keep_days: i64) {
        if keep_days <= 0 { return; }
        let cutoff = Utc::now() - chrono::Duration::days(keep_days);

        let kept = self.load_all().into_iter()
            .filter(|o| o.timestamp >= cutoff)
            .collect::<Vec<_>>();

        let mut inner = self.inner.lock().unwrap();

        if let Some(dir) = Path::new(&self.path).parent() {
            let _ = std::fs::create_dir_all(dir);
        }

        // Перезаписать файл только сохранёнными записями
        if let Ok(mut file) = std::fs::File::create(&self.path) {
            use std::io::Write;
            // Сортируем по времени
            let mut sorted = kept.clone();
            sorted.sort_by_key(|o| o.timestamp);
            for o in &sorted {
                if let Ok(json) = serde_json::to_string(o) {
                    let _ = writeln!(file, "{}", json);
                }
            }
        }

        inner.cache = Some(kept);
    }

    /// Очистить кэш (например, после внешней модификации файла).
    pub fn invalidate_cache(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.cache = None;
    }

    /// Сбросить все данные.
    pub fn reset_all(&self) {
        let _ = std::fs::remove_file(&self.path);
        let mut inner = self.inner.lock().unwrap();
        inner.cache = Some(Vec::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_outcome(genome_id: &str, network: &str, score: u32) -> ProbeOutcome {
        let mut o = ProbeOutcome::new(genome_id.into(), network.into());
        o.score = score;
        o.timestamp = Utc::now();
        o
    }

    #[test]
    fn test_history_create() {
        let h = AiHistoryStore::new("/tmp/bsdpi-test-history.json".into());
        assert!(h.load_all().is_empty());
    }

    #[test]
    fn test_append_and_load() {
        let path = "/tmp/bsdpi-test-append.json";
        let _ = std::fs::remove_file(path);

        let h = AiHistoryStore::new(path.into());
        h.append(make_outcome("g1", "net1", 85));
        h.append(make_outcome("g2", "net1", 50));

        let all = h.load_all();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].genome_id, "g1");
        assert_eq!(all[1].genome_id, "g2");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_load_for_network() {
        let path = "/tmp/bsdpi-test-load-net.json";
        let _ = std::fs::remove_file(path);

        let h = AiHistoryStore::new(path.into());
        h.append(make_outcome("g1", "net1", 85));
        h.append(make_outcome("g1", "net2", 90));
        h.append(make_outcome("g2", "net1", 50));

        let net1 = h.load_for_network("net1");
        assert_eq!(net1.len(), 2);
        assert!(net1.iter().all(|o| o.network_hash == "net1"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_load_for_genome_and_network() {
        let path = "/tmp/bsdpi-test-load-gen.json";
        let _ = std::fs::remove_file(path);

        let h = AiHistoryStore::new(path.into());
        h.append(make_outcome("g1", "net1", 85));
        h.append(make_outcome("g1", "net2", 90));
        h.append(make_outcome("g2", "net1", 50));

        let results = h.load_for("g1", "net1");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].genome_id, "g1");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_cache_invalidation() {
        let path = "/tmp/bsdpi-test-cache.json";
        let _ = std::fs::remove_file(path);

        let h = AiHistoryStore::new(path.into());
        h.append(make_outcome("g1", "net1", 85));
        assert_eq!(h.load_all().len(), 1);

        h.invalidate_cache();
        // После инвалидации кэша, load_all перечитает с диска
        assert_eq!(h.load_all().len(), 1);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_reset() {
        let path = "/tmp/bsdpi-test-reset.json";
        let _ = std::fs::remove_file(path);

        let h = AiHistoryStore::new(path.into());
        h.append(make_outcome("g1", "net1", 85));
        assert_eq!(h.load_all().len(), 1);

        h.reset_all();
        assert_eq!(h.load_all().len(), 0);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_rotate_old_entries() {
        let path = "/tmp/bsdpi-test-rotate.json";
        let _ = std::fs::remove_file(path);

        let h = AiHistoryStore::new(path.into());

        // Старая запись
        let mut old = make_outcome("g1", "net1", 85);
        old.timestamp = Utc::now() - chrono::Duration::days(10);
        h.append(old);

        // Новая запись
        h.append(make_outcome("g2", "net1", 90));

        h.rotate_old_entries(5); // keep 5 days

        let all = h.load_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].genome_id, "g2");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_history_record_creation() {
        let record = HistoryRecord::new(
            WorkEvent::StrategySelected("s1".into()),
            WorkResult::Success,
        );
        assert_eq!(
            match record.event {
                WorkEvent::StrategySelected(ref id) => id.clone(),
                _ => String::new(),
            },
            "s1"
        );
    }

    #[test]
    fn test_empty_file_load() {
        let path = "/tmp/bsdpi-test-empty.json";
        let _ = std::fs::remove_file(path);
        let h = AiHistoryStore::new(path.into());
        assert!(h.load_all().is_empty());
    }

    #[test]
    fn test_probe_outcome_fields() {
        let mut o = ProbeOutcome::new("g1".into(), "net1".into());
        o.score = 100;
        o.success_rate = 1.0;
        o.avg_latency_ms = 42.5;
        o.process_stable = true;
        o.failed_target_keys.push("example.com".into());
        o.failure_signature = Some("timeout".into());

        assert_eq!(o.genome_id, "g1");
        assert_eq!(o.network_hash, "net1");
        assert_eq!(o.score, 100);
        assert!((o.success_rate - 1.0).abs() < 0.01);
        assert!(o.process_stable);
    }
}
