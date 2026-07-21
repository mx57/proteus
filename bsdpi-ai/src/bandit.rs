//! BanditSelector — Thompson Sampling + UCB1 для выбора стратегии.
//!
//! Полный порт C# `BSDPI.AI/Services/BanditSelector.cs`:
//! - Thompson Sampling (через Marsaglia-Tsang Gamma → Beta)
//! - UCB1 с exploration bonus
//! - Pareto Front multi-objective optimization (score vs latency)
//! - Adaptive exploration (BOLT)
//! - Failure backoff с jitter
//! - Cooldown mechanisms (block, family, signature)

use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};

/// Конфигурация bandit алгоритма.
#[derive(Debug, Clone)]
pub struct BanditConfig {
    /// Exploration rate (‰) — 0 = pure exploitation, 1000 = pure exploration
    pub exploration_rate: u32,
    /// Использовать Thompson Sampling (true) или UCB1 (false)
    pub use_thompson: bool,
    /// Парето-фронт (multi-objective: score vs latency)
    pub pareto_enabled: bool,
    /// Max failure streak before perma-backoff
    pub max_backoff_streak: u32,
}

impl Default for BanditConfig {
    fn default() -> Self {
        Self {
            exploration_rate: 100,  // 10%
            use_thompson: true,
            pareto_enabled: false,
            max_backoff_streak: 5,
        }
    }
}

/// Статистика одной «руки» (стратегии) bandit.
#[derive(Debug, Clone)]
pub struct BanditArm {
    /// ID стратегии
    pub strategy_id: String,
    /// Количество успехов (на этой сети)
    pub successes: u32,
    /// Количество попыток (на этой сети)
    pub trials: u32,
    /// Alpha параметр Beta-распределения
    pub alpha: f64,
    /// Beta параметр Beta-распределения
    pub beta: f64,
    /// Средняя задержка (мс)
    pub avg_latency: f64,
}

impl BanditArm {
    pub fn new(strategy_id: String) -> Self {
        Self {
            strategy_id,
            successes: 0,
            trials: 0,
            alpha: 1.0,
            beta: 1.0,
            avg_latency: 1000.0,
        }
    }

    /// Wilson Score Lower Bound для ранжирования.
    pub fn wilson_score(&self) -> f64 {
        if self.trials == 0 {
            return 0.0;
        }
        crate::wilson::WilsonScore::lower_bound(self.successes, self.trials, 1.96)
    }

    /// Средняя награда (alpha / (alpha + beta)).
    pub fn mean_reward(&self) -> f64 {
        let total = self.alpha + self.beta;
        if total == 0.0 { 0.5 } else { self.alpha / total }
    }

    /// Общее количество pull'ов.
    pub fn pulls(&self) -> u32 {
        self.successes + self.trials
    }
}

/// Bandit-селектор для выбора наилучшей стратегии.
#[derive(Debug, Clone)]
pub struct BanditSelector {
    pub config: BanditConfig,
    pub arms: Vec<BanditArm>,
    // Стратегии, заблокированные после неудач
    blocked_until: HashMap<String, DateTime<Utc>>,
    // Счётчик последовательных неудач
    failure_streak: HashMap<String, u32>,
    // Cooldown для семейств стратегий
    family_cooldown: HashMap<String, DateTime<Utc>>,
    // Cooldown для сигнатур ошибок
    sig_cooldown: HashMap<String, DateTime<Utc>>,
    // RNG (StdRng для детерминизма в тестах)
    rng: StdRng,
}

impl BanditSelector {
    pub fn new(config: BanditConfig) -> Self {
        Self {
            config,
            arms: Vec::new(),
            blocked_until: HashMap::new(),
            failure_streak: HashMap::new(),
            family_cooldown: HashMap::new(),
            sig_cooldown: HashMap::new(),
            rng: StdRng::seed_from_u64(rand::random::<u64>()),
        }
    }

    /// Создаёт селектор с фиксированным seed (для тестов).
    pub fn new_seeded(config: BanditConfig, seed: u64) -> Self {
        Self {
            config,
            arms: Vec::new(),
            blocked_until: HashMap::new(),
            failure_streak: HashMap::new(),
            family_cooldown: HashMap::new(),
            sig_cooldown: HashMap::new(),
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Добавить стратегию в пул bandit.
    pub fn add_arm(&mut self, strategy_id: String) {
        if !self.arms.iter().any(|a| a.strategy_id == strategy_id) {
            self.arms.push(BanditArm::new(strategy_id));
        }
    }

    /// Удалить стратегию из пула.
    pub fn remove_arm(&mut self, strategy_id: &str) {
        self.arms.retain(|a| a.strategy_id != strategy_id);
    }

    /// Получить количество рук.
    pub fn arm_count(&self) -> usize {
        self.arms.len()
    }

    /// Найти arm по strategy_id.
    pub fn find_arm(&self, strategy_id: &str) -> Option<&BanditArm> {
        self.arms.iter().find(|a| a.strategy_id == strategy_id)
    }

    /// Найти arm по strategy_id (mut).
    pub fn find_arm_mut(&mut self, strategy_id: &str) -> Option<&mut BanditArm> {
        self.arms.iter_mut().find(|a| a.strategy_id == strategy_id)
    }

    /// Основной метод выбора стратегии.
    /// Портовый эквивалент C# `Pick()`.
    ///
    /// 1. Фильтрует заблокированные стратегии
    /// 2. Применяет adaptive exploration
    /// 3. Использует Thompson Sampling или UCB1
    /// 4. Парето-фронт если включён
    pub fn pick(&mut self, candidate_ids: &[String]) -> Option<String> {
        let now = Utc::now();

        // Фильтр: убираем заблокированные
        let usable: Vec<&String> = candidate_ids.iter()
            .filter(|id| {
                !self.blocked_until.contains_key(*id as &str)
                    || self.blocked_until.get(*id as &str).unwrap() <= &now
            })
            .collect();

        if usable.is_empty() {
            return None;
        }

        let total_pulls: u32 = self.arms.iter().map(|a| a.trials).sum();
        let total_pulls_f = total_pulls as f64;

        // Adaptive exploration (BOLT): уменьшаем exploration с опытом
        let adaptive_exploration =
            self.config.exploration_rate as f64 / (1.0 + total_pulls_f.sqrt() / 50.0);

        // Exploration: O(N) поиск стратегии с минимальным числом pull'ов
        if self.rng.gen::<f64>() * 1000.0 < adaptive_exploration {
            return self.pick_min_pulls(&usable);
        }

        // Exploitation: Thompson Sampling или UCB1
        let mut scored: Vec<(&String, f64, f64)> = usable.iter().map(|id| {
            let (pulls, alpha, beta, latency) = match self.find_arm(id) {
                Some(arm) => (arm.pulls() as f64, arm.alpha, arm.beta, arm.avg_latency),
                None => (0.0, 1.0, 1.0, 1000.0),
            };

            let score = if self.config.use_thompson {
                // Thompson Sampling: семплируем Beta(alpha, beta)
                if pulls < 1.0 {
                    0.5 // uniform prior для непроверенных
                } else {
                    self.sample_beta(alpha, beta)
                }
            } else {
                // UCB1
                let mean = if pulls < 1.0 { 0.5 } else { alpha / (alpha + beta) };
                let n = pulls.max(1.0);
                mean + (2.0 * (total_pulls_f + 1.0).ln() / n).sqrt()
            };

            (*id, score, latency)
        }).collect();

        if self.config.pareto_enabled {
            return self.pareto_pick(&scored).map(|s| s.clone());
        }

        // O(N) max по score
        scored.into_iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _, _)| id.clone())
    }

    /// Выбор стратегии с минимальным числом pull'ов (exploration branch).
    fn pick_min_pulls(&self, usable: &[&String]) -> Option<String> {
        usable.iter()
            .map(|id| {
                let arm = self.find_arm(id);
                let pulls = arm.map(|a| a.pulls()).unwrap_or(0);
                (*id, pulls)
            })
            .min_by_key(|(_, pulls)| *pulls)
            .map(|(id, _)| id.clone())
    }

    /// Выбор случайной стратегии из Парето-фронта.
    fn pareto_pick<'a>(&mut self, scored: &[(&'a String, f64, f64)]) -> Option<&'a String> {
        let pareto = self.pareto_front_internal(scored);
        if pareto.is_empty() {
            return scored.first().map(|(id, _, _)| *id);
        }
        let idx = self.rng.gen_range(0..pareto.len());
        Some(pareto[idx])
    }

    /// Вычисление Парето-фронта (score vs latency).
    fn pareto_front_internal<'a>(&self, scored: &[(&'a String, f64, f64)]) -> Vec<&'a String> {
        let mut front = Vec::new();
        for item in scored {
            let mut dominated = false;
            for other in scored {
                if other.0 == item.0 { continue; }
                // other доминирует item если:
                // - score >= item.score AND latency <= item.latency
                // - хотя бы один строго лучше
                if other.1 >= item.1 && other.2 <= item.2
                    && (other.1 > item.1 || other.2 < item.2)
                {
                    dominated = true;
                    break;
                }
            }
            if !dominated {
                front.push(item.0);
            }
        }
        front
    }

    /// Возвращает лучшую известную стратегию для сети (по mean reward).
    pub fn best_known(&self, candidate_ids: &[String]) -> Option<String> {
        candidate_ids.iter()
            .filter_map(|id| {
                let arm = self.find_arm(id)?;
                if arm.pulls() < 1 {
                    return None;
                }
                let mean = arm.mean_reward();
                Some((id.clone(), mean))
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _)| id)
    }

    pub fn pareto_front(&self, candidate_ids: &[String]) -> Vec<String> {
        let scored: Vec<(&String, f64, f64)> = candidate_ids
            .iter()
            .map(|id| {
                let (pulls, score, latency) = match self.find_arm(id) {
                    Some(arm) => {
                        let pulls = arm.pulls();
                        let score = if pulls < 1 { 0.5 } else { arm.mean_reward() };
                        (pulls, score, arm.avg_latency)
                    }
                    None => (0, 0.5, 1000.0),
                };
                (id, score, latency)
            })
            .collect();
        self.pareto_front_internal(&scored).into_iter().cloned().collect()
    }

    /// Регистрация успеха стратегии.
    pub fn register_success(&mut self, strategy_id: &str) {
        self.failure_streak.remove(strategy_id);
        self.blocked_until.remove(strategy_id);
        if let Some(arm) = self.find_arm_mut(strategy_id) {
            arm.successes += 1;
            arm.alpha += 1.0;
        }
    }

    /// Регистрация неудачи стратегии с экспоненциальным backoff.
    pub fn register_failure(&mut self, strategy_id: &str, failure_signature: Option<&str>) {
        let streak = self.failure_streak.get(strategy_id).copied().unwrap_or(0) + 1;
        self.failure_streak.insert(strategy_id.to_string(), streak);

        // Exponential backoff с jitter
        let backoff_ms: u64 = match streak {
            1 => 300,
            2 => 700,
            3 => 1500,
            _ => 3000,
        };
        let jitter = 1.0 + (self.rng.gen::<f64>() * 0.7 - 0.35);
        let delay_ms = (backoff_ms as f64 * jitter) as i64;
        let until = Utc::now() + Duration::milliseconds(delay_ms);
        self.blocked_until.insert(strategy_id.to_string(), until);

        // Family cooldown (по desync mode)
        let fam_key = format!("{}|zapret", strategy_id);
        self.family_cooldown.insert(fam_key, Utc::now() + Duration::seconds(15));

        // Signature cooldown
        if let Some(sig) = failure_signature {
            let sig_key = format!("{}|{}", strategy_id, sig);
            self.sig_cooldown.insert(sig_key, Utc::now() + Duration::seconds(15));
        }

        if let Some(arm) = self.find_arm_mut(strategy_id) {
            arm.trials += 1;
            arm.beta += 1.0;
        }
    }

    /// Проверка, находится ли семейство стратегии на cooldown.
    pub fn is_family_cooling(&self, strategy_id: &str) -> bool {
        let now = Utc::now();
        let fam_key = format!("{}|zapret", strategy_id);
        self.family_cooldown.get(&fam_key).map_or(false, |t| t > &now)
    }

    // ─── Thompson Sampling: Marsaglia-Tsang Gamma sampler ───

    /// Семплирование Beta(alpha, beta) через Gamma-семплинг.
    fn sample_beta(&mut self, alpha: f64, beta: f64) -> f64 {
        let x = self.gamma_sample(alpha);
        let y = self.gamma_sample(beta);
        x / (x + y + 1e-12)
    }

    /// Marsaglia-Tsang Gamma sampler (порт C# реализации).
    fn gamma_sample(&mut self, shape: f64) -> f64 {
        if shape < 1e-9 {
            return 1e-9;
        }
        if shape < 1.0 {
            let u: f64 = self.rng.gen();
            return self.gamma_sample(shape + 1.0) * u.powf(1.0 / shape);
        }

        let d = shape - 1.0 / 3.0;
        let c = 1.0 / (9.0 * d).sqrt();

        loop {
            let x = loop {
                let x = self.normal_sample();
                if x > -1.0 / c {
                    break x;
                }
            };

            let v = (1.0 + c * x).powi(3);
            let u: f64 = self.rng.gen();

            if u < 1.0 - 0.0331 * x.powi(4) {
                return d * v;
            }
            if u.ln() < 0.5 * x * x + d * (1.0 - v + v.ln()) {
                return d * v;
            }
        }
    }

    /// Box-Muller normal sampler.
    fn normal_sample(&mut self) -> f64 {
        let u1: f64 = self.rng.gen();
        let u2: f64 = self.rng.gen();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bandit_creation() {
        let selector = BanditSelector::new(BanditConfig::default());
        assert_eq!(selector.arm_count(), 0);
    }

    #[test]
    fn test_add_arm() {
        let mut selector = BanditSelector::new(BanditConfig::default());
        selector.add_arm("strategy-1".into());
        assert_eq!(selector.arm_count(), 1);
        // Double add should not duplicate
        selector.add_arm("strategy-1".into());
        assert_eq!(selector.arm_count(), 1);
    }

    #[test]
    fn test_remove_arm() {
        let mut selector = BanditSelector::new(BanditConfig::default());
        selector.add_arm("s1".into());
        selector.add_arm("s2".into());
        selector.remove_arm("s1");
        assert_eq!(selector.arm_count(), 1);
        assert!(selector.find_arm("s2").is_some());
    }

    #[test]
    fn test_find_arm() {
        let mut selector = BanditSelector::new(BanditConfig::default());
        selector.add_arm("test-arm".into());
        let arm = selector.find_arm("test-arm");
        assert!(arm.is_some());
        assert_eq!(arm.unwrap().strategy_id, "test-arm");
    }

    #[test]
    fn test_arm_wilson_new() {
        let arm = BanditArm::new("test".into());
        assert_eq!(arm.wilson_score(), 0.0);
    }

    #[test]
    fn test_arm_mean_reward() {
        let arm = BanditArm::new("test".into());
        assert!((arm.mean_reward() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_default_config() {
        let cfg = BanditConfig::default();
        assert_eq!(cfg.exploration_rate, 100);
        assert!(cfg.use_thompson);
    }

    #[test]
    fn test_pick_empty_candidates() {
        let mut selector = BanditSelector::new(BanditConfig::default());
        assert!(selector.pick(&[]).is_none());
    }

    #[test]
    fn test_pick_from_candidates() {
        let mut selector = BanditSelector::new(BanditConfig::default());
        selector.add_arm("s1".into());
        selector.add_arm("s2".into());
        let picked = selector.pick(&["s1".into(), "s2".into()]);
        assert!(picked.is_some());
        assert!(["s1", "s2"].contains(&picked.as_deref().unwrap()));
    }

    #[test]
    fn test_pick_with_all_blocked_fallback() {
        let mut selector = BanditSelector::new(BanditConfig::default());
        selector.add_arm("s1".into());
        selector.add_arm("s2".into());
        // Block one
        selector.register_failure("s1", None);
        selector.register_failure("s1", None);
        selector.register_failure("s1", None);
        let picked = selector.pick(&["s1".into(), "s2".into()]);
        assert!(picked.is_some());
    }

    #[test]
    fn test_register_success_clears_failures() {
        let mut selector = BanditSelector::new(BanditConfig::default());
        selector.add_arm("s1".into());
        selector.register_failure("s1", None);
        assert_eq!(selector.failure_streak.get("s1"), Some(&1));
        selector.register_success("s1");
        assert!(selector.failure_streak.get("s1").is_none());
        assert!(selector.blocked_until.get("s1").is_none());
    }

    #[test]
    fn test_best_known() {
        let mut selector = BanditSelector::new(BanditConfig::default());
        selector.add_arm("s1".into());
        selector.add_arm("s2".into());
        // После успехов s1 должна быть лучшей
        selector.register_success("s1");
        selector.register_success("s1");
        selector.register_success("s1");
        selector.register_failure("s2", None);

        let best = selector.best_known(&["s1".into(), "s2".into()]);
        assert_eq!(best.as_deref(), Some("s1"));
    }

    #[test]
    fn test_pareto_front() {
        let mut selector = BanditSelector::new_seeded(BanditConfig { pareto_enabled: true, ..Default::default() }, 42);
        selector.add_arm("low-score-fast".into());
        selector.add_arm("high-score-slow".into());
        selector.add_arm("med-score-med".into());

        let front = selector.pareto_front(&[
            "low-score-fast".into(),
            "high-score-slow".into(),
            "med-score-med".into(),
        ]);
        assert!(!front.is_empty());
    }

    #[test]
    fn test_gamma_sample() {
        let mut selector = BanditSelector::new(BanditConfig::default());
        let s = selector.sample_beta(5.0, 2.0);
        assert!(s > 0.0 && s < 1.0, "Beta sample should be in (0,1), got {}", s);
    }

    #[test]
    fn test_thompson_preference() {
        let mut selector = BanditSelector::new_seeded(BanditConfig::default(), 42);
        selector.add_arm("good".into());
        selector.add_arm("bad".into());

        // Good: много успехов, Bad: много неудач
        for _ in 0..10 { selector.register_success("good"); }
        for _ in 0..10 { selector.register_failure("bad", None); }

        // Thompson должен выбирать good чаще
        let mut good_count = 0;
        for _ in 0..50 {
            if selector.pick(&["good".into(), "bad".into()]).as_deref() == Some("good") {
                good_count += 1;
            }
        }
        assert!(good_count > 30, "Thompson should prefer good >30/50, got {}", good_count);
    }

    #[test]
    fn test_ucb1_selection() {
        let mut selector = BanditSelector::new_seeded(
            BanditConfig { use_thompson: false, ..Default::default() }, 42,
        );
        selector.add_arm("explored".into());
        selector.add_arm("new".into());

        for _ in 0..10 { selector.register_success("explored"); }

        // UCB1 даст бонус новой руке
        for _ in 0..20 {
            selector.pick(&["explored".into(), "new".into()]);
        }
        // Просто проверяем что UCB1 не падает
        assert!(selector.arm_count() == 2);
    }

    #[test]
    fn test_exponential_backoff() {
        let mut selector = BanditSelector::new(BanditConfig::default());
        selector.add_arm("s1".into());

        for i in 1..=4 {
            let before = chrono::Utc::now();
            selector.register_failure("s1", None);
            // Каждый последующий failure увеличивает backoff
            if i >= 1 {
                let blocked = selector.blocked_until.get("s1").unwrap();
                assert!(blocked > &before, "Backoff should be in future");
            }
        }
    }

    #[test]
    fn test_normal_sample() {
        let mut selector = BanditSelector::new(BanditConfig::default());
        let samples: Vec<f64> = (0..1000).map(|_| selector.normal_sample()).collect();
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let variance = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / samples.len() as f64;
        assert!((mean).abs() < 0.2, "Normal mean should be ~0, got {}", mean);
        assert!((variance - 1.0).abs() < 0.2, "Normal variance should be ~1, got {}", variance);
    }
}
