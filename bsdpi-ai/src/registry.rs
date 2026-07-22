//! AiStrategyRegistry — персистентное хранилище стратегий.
//!
//! Хранит геномы в JSON-файле. Поддерживает CRUD, поиск по ID,
//! снапшоты для bandit, агрегированную статистику.
//!
//! ## C# оригинал
//! `BSDPI.AI/Services/AiStrategyRegistry.cs`

use crate::error::AiError;
use crate::genome::{StrategyGenome, StrategyOrigin};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Модель данных для сериализации.
#[derive(Debug, Serialize, Deserialize)]
struct RegistryData {
    genomes: Vec<StrategyGenome>,
    generation_counter: u32,
}

impl Default for RegistryData {
    fn default() -> Self {
        Self {
            genomes: Vec::new(),
            generation_counter: 0,
        }
    }
}

/// Персистентный реестр стратегий.
///
/// Автоматически сохраняется в JSON файл при изменениях.
pub struct AiStrategyRegistry {
    data: RegistryData,
    file_path: PathBuf,
    lookup_by_id: HashMap<String, usize>,
    lookup_by_signature: HashMap<String, usize>,
    dirty: bool,
}

impl AiStrategyRegistry {
    /// Создать новый реестр с указанным файлом для хранения.
    pub fn new(file_path: impl Into<PathBuf>) -> Result<Self, AiError> {
        let path: PathBuf = file_path.into();
        let data = if path.exists() {
            let content = fs::read_to_string(&path)
                .map_err(|e| AiError::Registry(format!("cannot read {path:?}: {e}")))?;
            serde_json::from_str(&content)
                .map_err(|e| AiError::Registry(format!("cannot parse {path:?}: {e}")))?
        } else {
            // Создаём директорию если нужно
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            RegistryData::default()
        };

        let mut reg = Self {
            data,
            file_path: path,
            lookup_by_id: HashMap::new(),
            lookup_by_signature: HashMap::new(),
            dirty: false,
        };
        reg.rebuild_lookups();
        Ok(reg)
    }

    /// Создать реестр в памяти (без файла).
    pub fn in_memory() -> Self {
        Self {
            data: RegistryData::default(),
            file_path: PathBuf::new(),
            lookup_by_id: HashMap::new(),
            lookup_by_signature: HashMap::new(),
            dirty: false,
        }
    }

    // ========== Query ==========

    /// Получить все геномы.
    pub fn all_genomes(&self) -> &[StrategyGenome] {
        &self.data.genomes
    }

    /// Получить активные геномы (orchestrator_enabled = true).
    pub fn active_genomes(&self) -> Vec<&StrategyGenome> {
        self.data
            .genomes
            .iter()
            .filter(|g| g.orchestrator_enabled)
            .collect()
    }

    /// Получить геном по ID.
    pub fn get_by_id(&self, id: &Uuid) -> Option<&StrategyGenome> {
        self.lookup_by_id
            .get(&id.to_string())
            .map(|&idx| &self.data.genomes[idx])
    }

    /// Получить геномы по происхождению.
    pub fn get_by_origin(&self, origin: StrategyOrigin) -> Vec<&StrategyGenome> {
        self.data
            .genomes
            .iter()
            .filter(|g| g.origin == origin)
            .collect()
    }

    /// Текущий generation counter.
    pub fn generation_counter(&self) -> u32 {
        self.data.generation_counter
    }

    // ========== Mutations ==========

    /// Увеличить generation counter.
    pub fn increment_generation(&mut self) -> u32 {
        self.data.generation_counter += 1;
        self.dirty = true;
        self.data.generation_counter
    }

    /// Добавить или обновить геном.
    pub fn upsert(&mut self, genome: StrategyGenome) {
        let id = genome.id;

        if let Some(&idx) = self.lookup_by_id.get(&id.to_string()) {
            // Update existing
            self.data.genomes[idx] = genome.clone();
        } else {
            // Insert new
            self.data.genomes.push(genome.clone());
            self.lookup_by_id
                .insert(id.to_string(), self.data.genomes.len() - 1);
        }

        self.dirty = true;
    }

    /// Удалить геном по ID. Возвращает true если был удалён.
    pub fn remove(&mut self, id: &Uuid) -> bool {
        if let Some(&idx) = self.lookup_by_id.get(&id.to_string()) {
            self.data.genomes.swap_remove(idx);
            self.dirty = true;
            self.rebuild_lookups();
            true
        } else {
            false
        }
    }

    /// Загрузить builtin стратегии из директории.
    pub fn load_builtins(&mut self, builtins: Vec<StrategyGenome>) {
        for g in builtins {
            if !self.lookup_by_id.contains_key(&g.id.to_string()) {
                self.upsert(g);
            }
        }
    }

    /// Сохранить на диск (если dirty).
    pub fn save(&mut self) -> Result<(), AiError> {
        if !self.dirty || self.file_path.as_os_str().is_empty() {
            return Ok(());
        }

        // Создаём временный файл, затем переименовываем (атомарная запись)
        let tmp_path = self.file_path.with_extension("tmp");
        let json = serde_json::to_string_pretty(&self.data)
            .map_err(|e| AiError::Serialization(e.to_string()))?;

        fs::write(&tmp_path, &json)
            .map_err(|e| AiError::Registry(format!("cannot write {tmp_path:?}: {e}")))?;

        fs::rename(&tmp_path, &self.file_path)
            .map_err(|e| AiError::Registry(format!("cannot rename {tmp_path:?} -> {:?}: {e}", self.file_path)))?;

        self.dirty = false;
        Ok(())
    }

    /// Снять снапшот bandit-статистики для сети.
    /// Возвращает HashMap<genome_id, (alpha, beta, avg_latency)>
    pub fn get_bandit_snapshot(
        &self,
        outcomes: &[(Uuid, i32, f64)], // (genome_id, score, latency)
    ) -> HashMap<String, (f64, f64, f64)> {
        let mut snapshot: HashMap<String, Vec<&(Uuid, i32, f64)>> = HashMap::new();
        for outcome in outcomes {
            snapshot
                .entry(outcome.0.to_string())
                .or_default()
                .push(outcome);
        }

        snapshot
            .into_iter()
            .map(|(id, outcomes)| {
                let successes = outcomes.iter().filter(|(_, s, _)| *s >= 50).count() as f64;
                let failures = outcomes.len() as f64 - successes;
                let avg_latency = if outcomes.is_empty() {
                    1000.0
                } else {
                    outcomes.iter().map(|(_, _, l)| l).sum::<f64>() / outcomes.len() as f64
                };
                let alpha = 1.0 + successes;
                let beta = 1.0 + failures;
                (id, (alpha, beta, avg_latency))
            })
            .collect()
    }

    // ========== Internal ==========

    fn rebuild_lookups(&mut self) {
        self.lookup_by_id.clear();
        self.lookup_by_signature.clear();
        for (i, genome) in self.data.genomes.iter().enumerate() {
            self.lookup_by_id.insert(genome.id.to_string(), i);
            let sig = crate::signature::compute(genome);
            self.lookup_by_signature.insert(sig, i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::{DpiEngineType, StrategyOrigin};

    fn make_genome() -> StrategyGenome {
        StrategyGenome::new(DpiEngineType::Zapret, StrategyOrigin::Builtin)
    }

    #[test]
    fn test_registry_empty_initially() {
        let reg = AiStrategyRegistry::in_memory();
        assert!(reg.all_genomes().is_empty());
    }

    #[test]
    fn test_upsert_adds_genome() {
        let mut reg = AiStrategyRegistry::in_memory();
        let g = make_genome();
        reg.upsert(g);
        assert_eq!(reg.all_genomes().len(), 1);
    }

    #[test]
    fn test_upsert_updates_existing() {
        let mut reg = AiStrategyRegistry::in_memory();
        let mut g = make_genome();
        let id = g.id;
        g.display_name = "original".into();
        reg.upsert(g);

        let mut g2 = make_genome();
        g2.id = id;
        g2.display_name = "updated".into();
        reg.upsert(g2);

        assert_eq!(reg.all_genomes().len(), 1);
        assert_eq!(reg.get_by_id(&id).unwrap().display_name, "updated");
    }

    #[test]
    fn test_get_by_id() {
        let mut reg = AiStrategyRegistry::in_memory();
        let g = make_genome();
        let id = g.id;
        reg.upsert(g);
        assert!(reg.get_by_id(&id).is_some());
        assert!(reg.get_by_id(&Uuid::new_v4()).is_none());
    }

    #[test]
    fn test_remove_deletes() {
        let mut reg = AiStrategyRegistry::in_memory();
        let g = make_genome();
        let id = g.id;
        reg.upsert(g);
        assert!(reg.remove(&id));
        assert!(reg.get_by_id(&id).is_none());
    }

    #[test]
    fn test_active_genomes_filter() {
        let mut reg = AiStrategyRegistry::in_memory();
        let mut g1 = make_genome();
        g1.orchestrator_enabled = true;
        let mut g2 = make_genome();
        g2.orchestrator_enabled = false;

        reg.upsert(g1);
        reg.upsert(g2);

        assert_eq!(reg.active_genomes().len(), 1);
    }

    #[test]
    fn test_generation_counter() {
        let mut reg = AiStrategyRegistry::in_memory();
        assert_eq!(reg.generation_counter(), 0);
        let gen = reg.increment_generation();
        assert_eq!(gen, 1);
        assert_eq!(reg.generation_counter(), 1);
    }

    #[test]
    fn test_get_by_origin() {
        let mut reg = AiStrategyRegistry::in_memory();
        reg.upsert(make_genome());
        let mut evolved = make_genome();
        evolved.origin = StrategyOrigin::Evolved;
        reg.upsert(evolved);

        assert_eq!(reg.get_by_origin(StrategyOrigin::Builtin).len(), 1);
        assert_eq!(reg.get_by_origin(StrategyOrigin::Evolved).len(), 1);
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("registry.json");

        let mut reg = AiStrategyRegistry::new(&file_path).unwrap();
        reg.upsert(make_genome());
        reg.save().unwrap();

        // Загружаем снова
        let reg2 = AiStrategyRegistry::new(&file_path).unwrap();
        assert_eq!(reg2.all_genomes().len(), 1);
    }

    #[test]
    fn test_bandit_snapshot() {
        let reg = AiStrategyRegistry::in_memory();
        let g = make_genome();
        let outcomes = vec![(g.id, 80, 50.0), (g.id, 30, 200.0), (g.id, 90, 100.0)];
        let snapshot = reg.get_bandit_snapshot(&outcomes);

        let (alpha, beta, _) = snapshot.get(&g.id.to_string()).unwrap();
        assert!(*alpha >= 2.0); // 2 успеха => alpha = 3
        assert!(*beta >= 1.0); // 1 неудача => beta = 2
    }
}
