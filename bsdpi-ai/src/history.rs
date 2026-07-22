//! AiHistoryStore — append-only лог истории испытаний.
//!
//! Хранит каждое испытание как JSONL строку. Поддерживает загрузку
//! по сети, по геному, агрегированную статистику.
//!
//! ## C# оригинал
//! `BSDPI.AI/Services/AiHistoryStore.cs`

use crate::error::AiError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Одна запись истории испытания.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryRecord {
    /// ID генома, который тестировался
    pub genome_id: Uuid,
    /// Хеш сети
    pub network_hash: String,
    /// Оценка успеха (0-100)
    pub score: i32,
    /// Задержка в ms
    pub latency_ms: f64,
    /// Отметка времени
    pub timestamp: DateTime<Utc>,
    /// Произвольные метаданные (стратегия, ошибка и т.д.)
    pub metadata: HashMap<String, String>,
}

impl HistoryRecord {
    pub fn new(
        genome_id: Uuid,
        network_hash: impl Into<String>,
        score: i32,
        latency_ms: f64,
    ) -> Self {
        Self {
            genome_id,
            network_hash: network_hash.into(),
            score,
            latency_ms,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Хранилище истории испытаний.
///
/// Использует JSONL формат (одна JSON строка на запись).
/// Автоматически ротирует файл при превышении лимита (по умолчанию 100K записей).
pub struct AiHistoryStore {
    file_path: PathBuf,
    /// Записи в памяти (in-memory cache)
    records: Vec<HistoryRecord>,
    /// Максимальное число записей до ротации
    max_records: usize,
    dirty: bool,
}

impl AiHistoryStore {
    /// Создать новое хранилище.
    pub fn new(file_path: impl Into<PathBuf>, max_records: usize) -> Result<Self, AiError> {
        let path: PathBuf = file_path.into();

        let records = if path.exists() {
            Self::load_from_file(&path)?
        } else {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            Vec::new()
        };

        Ok(Self {
            file_path: path,
            records,
            max_records,
            dirty: false,
        })
    }

    /// Создать in-memory хранилище (без файла).
    pub fn in_memory() -> Self {
        Self {
            file_path: PathBuf::new(),
            records: Vec::new(),
            max_records: 100_000,
            dirty: false,
        }
    }

    // ========== Write ==========

    /// Добавить запись.
    pub fn append(&mut self, record: HistoryRecord) -> Result<(), AiError> {
        // Добавляем в память
        self.records.push(record.clone());
        self.dirty = true;

        // Append в JSONL файл
        if !self.file_path.as_os_str().is_empty() {
            let line = serde_json::to_string(&record)
                .map_err(|e| AiError::History(format!("serialization: {e}")))?;

            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.file_path)
                .map_err(|e| AiError::History(format!("cannot open {:?}: {e}", self.file_path)))?;

            writeln!(file, "{line}")
                .map_err(|e| AiError::History(format!("cannot write: {e}")))?;
        }

        // Ротация если превышен лимит
        if self.records.len() > self.max_records {
            self.rotate()?;
        }

        Ok(())
    }

    // ========== Query ==========

    /// Все записи.
    pub fn all(&self) -> &[HistoryRecord] {
        &self.records
    }

    /// Записи для конкретной сети.
    pub fn for_network(&self, network_hash: &str) -> Vec<&HistoryRecord> {
        self.records
            .iter()
            .filter(|r| r.network_hash == network_hash)
            .collect()
    }

    /// Записи для конкретного генома.
    pub fn for_genome(&self, genome_id: &Uuid) -> Vec<&HistoryRecord> {
        self.records
            .iter()
            .filter(|r| r.genome_id == *genome_id)
            .collect()
    }

    /// Записи для генома на конкретной сети.
    pub fn for_genome_on_network(&self, genome_id: &Uuid, network_hash: &str) -> Vec<&HistoryRecord> {
        self.records
            .iter()
            .filter(|r| r.genome_id == *genome_id && r.network_hash == network_hash)
            .collect()
    }

    /// Получить `(genome_id, score)` пары для эволюции.
    pub fn outcomes(&self) -> Vec<(Uuid, i32)> {
        self.records
            .iter()
            .map(|r| (r.genome_id, r.score))
            .collect()
    }

    /// Получить outcomes с latency для bandit.
    pub fn outcomes_with_latency(&self) -> Vec<(Uuid, i32, f64)> {
        self.records
            .iter()
            .map(|r| (r.genome_id, r.score, r.latency_ms))
            .collect()
    }

    /// Записи для сети, как `(genome_id, score)`.
    pub fn outcomes_for_network(&self, network_hash: &str) -> Vec<(Uuid, i32)> {
        self.records
            .iter()
            .filter(|r| r.network_hash == network_hash)
            .map(|r| (r.genome_id, r.score))
            .collect()
    }

    /// Общее количество записей.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Очистить все записи.
    pub fn clear(&mut self) -> Result<(), AiError> {
        self.records.clear();
        self.dirty = true;

        if !self.file_path.as_os_str().is_empty() {
            fs::write(&self.file_path, "")
                .map_err(|e| AiError::History(format!("cannot clear: {e}")))?;
        }

        Ok(())
    }

    // ========== Internal ==========

    fn load_from_file(path: &Path) -> Result<Vec<HistoryRecord>, AiError> {
        let file = fs::File::open(path)
            .map_err(|e| AiError::History(format!("cannot open {path:?}: {e}")))?;

        let reader = BufReader::new(file);
        let mut records = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| {
                AiError::History(format!("read error at line {}: {e}", line_num + 1))
            })?;

            if line.trim().is_empty() {
                continue;
            }

            let record: HistoryRecord = serde_json::from_str(&line).map_err(|e| {
                AiError::History(format!(
                    "parse error at line {}: {e} — line starts with: {:?}",
                    line_num + 1,
                    &line[..line.len().min(80)]
                ))
            })?;

            records.push(record);
        }

        Ok(records)
    }

    fn rotate(&mut self) -> Result<(), AiError> {
        // Оставляем только последние max_records/2 записей
        let keep = self.max_records / 2;
        if self.records.len() > keep {
            self.records.drain(0..self.records.len() - keep);
        }

        // Переписываем файл
        if !self.file_path.as_os_str().is_empty() {
            let mut file = fs::File::create(&self.file_path)
                .map_err(|e| AiError::History(format!("cannot create for rotation: {e}")))?;

            for record in &self.records {
                let line = serde_json::to_string(record)
                    .map_err(|e| AiError::History(format!("serialization: {e}")))?;
                writeln!(file, "{line}")
                    .map_err(|e| AiError::History(format!("write error: {e}")))?;
            }
        }

        self.dirty = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::{DpiEngineType, StrategyGenome, StrategyOrigin};

    fn make_record(score: i32) -> HistoryRecord {
        let g = StrategyGenome::new(DpiEngineType::Zapret, StrategyOrigin::Builtin);
        HistoryRecord::new(g.id, "test-net", score, 100.0)
    }

    #[test]
    fn test_empty_store() {
        let store = AiHistoryStore::in_memory();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_append_record() {
        let mut store = AiHistoryStore::in_memory();
        let record = make_record(80);
        store.append(record).unwrap();
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_for_network() {
        let mut store = AiHistoryStore::in_memory();
        let mut r1 = make_record(80);
        r1.network_hash = "net-a".into();
        let mut r2 = make_record(30);
        r2.network_hash = "net-b".into();

        store.append(r1).unwrap();
        store.append(r2).unwrap();

        assert_eq!(store.for_network("net-a").len(), 1);
        assert_eq!(store.for_network("net-b").len(), 1);
    }

    #[test]
    fn test_outcomes() {
        let mut store = AiHistoryStore::in_memory();
        store.append(make_record(80)).unwrap();
        store.append(make_record(30)).unwrap();

        let outcomes = store.outcomes();
        assert_eq!(outcomes.len(), 2);
        assert!(outcomes.iter().any(|(_, s)| *s == 80));
        assert!(outcomes.iter().any(|(_, s)| *s == 30));
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.jsonl");

        let mut store = AiHistoryStore::new(&path, 1000).unwrap();
        store.append(make_record(80)).unwrap();

        // Загружаем снова
        let loaded = AiHistoryStore::new(&path, 1000).unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut store = AiHistoryStore::in_memory();
        store.append(make_record(80)).unwrap();
        store.clear().unwrap();
        assert!(store.is_empty());
    }

    #[test]
    fn test_for_genome_on_network() {
        let mut store = AiHistoryStore::in_memory();
        let g1 = StrategyGenome::new(DpiEngineType::Zapret, StrategyOrigin::Builtin);
        let g2 = StrategyGenome::new(DpiEngineType::ByeDpi, StrategyOrigin::Builtin);

        store.append(HistoryRecord::new(g1.id, "net-a", 80, 50.0)).unwrap();
        store.append(HistoryRecord::new(g1.id, "net-b", 30, 200.0)).unwrap();
        store.append(HistoryRecord::new(g2.id, "net-a", 95, 100.0)).unwrap();

        assert_eq!(store.for_genome_on_network(&g1.id, "net-a").len(), 1);
        assert_eq!(store.for_genome_on_network(&g1.id, "net-b").len(), 1);
        assert_eq!(store.for_genome_on_network(&g2.id, "net-a").len(), 1);
        assert_eq!(store.for_genome_on_network(&g2.id, "net-b").len(), 0);
    }

    #[test]
    fn test_rotation_keeps_recent() {
        let mut store = AiHistoryStore::in_memory();
        store.max_records = 10;

        for i in 0..20 {
            store.append(HistoryRecord::new(
                Uuid::new_v4(),
                format!("net-{i}"),
                i,
                i as f64,
            )).unwrap();
        }

        // Должно быть не больше max_records
        assert!(store.len() <= 10);
    }
}
