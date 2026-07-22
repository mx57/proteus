//! Multi-armed bandit selector — Thompson Sampling + UCB1 + Pareto front.
//!
//! ## C# оригинал
//! `BSDPI.AI/Services/BanditSelector.cs`

use crate::error::AiError;
use crate::genome::StrategyGenome;
use chrono::{DateTime, Utc};
use rand::Rng;
use std::collections::HashMap;

fn backoff_ms(streak: u32) -> u64 {
    match streak {
        1 => 300,
        2 => 700,
        3 => 1500,
        _ => 3000,
    }
}

/// Bandit arm — Beta-распределение для стратегии.
#[derive(Debug, Clone)]
pub struct BanditArm {
    pub alpha: f64,
    pub beta: f64,
    pub avg_latency: f64,
}

impl BanditArm {
    pub fn pulls(&self) -> f64 {
        self.alpha + self.beta - 2.0
    }

    pub fn mean(&self) -> f64 {
        let total = self.alpha + self.beta;
        if total == 0.0 {
            return 0.5;
        }
        self.alpha / total
    }

    pub fn new() -> Self {
        Self {
            alpha: 1.0,
            beta: 1.0,
            avg_latency: 1000.0,
        }
    }

    pub fn update(&mut self, success: bool, latency_ms: f64) {
        if success {
            self.alpha += 1.0;
        } else {
            self.beta += 1.0;
        }
        let n = self.pulls();
        if n > 1.0 {
            let weight = 2.0 / (n + 1.0);
            self.avg_latency = (1.0 - weight) * self.avg_latency + weight * latency_ms;
        } else {
            self.avg_latency = latency_ms;
        }
    }
}

impl Default for BanditArm {
    fn default() -> Self {
        Self::new()
    }
}

/// Результат оценки стратегии (для Pareto front).
#[derive(Debug, Clone)]
struct ScoredCandidate {
    index: usize,
    score: f64,
    latency: f64,
}

/// Многорукий бандит.
pub struct BanditSelector {
    local_bandits: HashMap<String, HashMap<String, BanditArm>>,
    global_arms: HashMap<String, BanditArm>,
    blocked_until: HashMap<String, DateTime<Utc>>,
    failure_streak: HashMap<String, u32>,
    rng: rand::rngs::ThreadRng,
}

impl BanditSelector {
    pub fn new() -> Self {
        Self {
            local_bandits: HashMap::new(),
            global_arms: HashMap::new(),
            blocked_until: HashMap::new(),
            failure_streak: HashMap::new(),
            rng: rand::thread_rng(),
        }
    }

    /// Выбрать лучшую стратегию.
    pub fn pick<'a>(
        &mut self,
        candidates: &'a [StrategyGenome],
        network_hash: &str,
        exploration_permil: u32,
        use_ucb1: bool,
        pareto_enabled: bool,
        total_pulls_on_network: f64,
    ) -> Result<&'a StrategyGenome, AiError> {
        if candidates.is_empty() {
            return Err(AiError::NoCandidates);
        }

        let now = Utc::now();

        // Шаг 1: найти usable (не заблокированные)
        let usable_indices: Vec<usize> = (0..candidates.len())
            .filter(|i| {
                !self
                    .blocked_until
                    .get(&candidates[*i].id.to_string())
                    .is_some_and(|u| *u > now)
            })
            .collect();

        if usable_indices.is_empty() {
            return Err(AiError::NoCandidates);
        }

        let local = self
            .local_bandits
            .entry(network_hash.to_string())
            .or_default();

        // Шаг 2: Adaptive exploration
        let adaptive_exploration =
            exploration_permil as f64 / (1.0 + (total_pulls_on_network / 50.0).sqrt());

        if (self.rng.gen::<f64>() * 1000.0) < adaptive_exploration {
            let mut min_pulls = f64::MAX;
            let mut best_idx = usable_indices[0];
            for &i in &usable_indices {
                let g = &candidates[i];
                let arm = local.get(&g.id.to_string());
                let pulls = arm.map_or(0.0, |a| a.pulls());
                if pulls < min_pulls {
                    min_pulls = pulls;
                    best_idx = i;
                }
            }
            return Ok(&candidates[best_idx]);
        }

        // Шаг 3: Exploitation — считаем скоры для usable
        let mut scored: Vec<ScoredCandidate> = usable_indices
            .iter()
            .map(|&i| {
                let g = &candidates[i];
                let arm = local.get(&g.id.to_string());
                let pulls = arm.map_or(0.0, |a| a.pulls());
                let score = if use_ucb1 {
                    let mean = arm.map_or(0.5, |a| a.mean());
                    let n = pulls.max(1.0);
                    mean + (2.0 * (total_pulls_on_network + 1.0).ln() / n).sqrt()
                } else {
                    if pulls < 0.5 { 0.5 } else {
                        let (alpha, beta) = arm.map_or((1.0, 1.0), |a| (a.alpha, a.beta));
                        sample_beta(&mut self.rng, alpha, beta)
                    }
                };
                let latency = arm.map_or(1000.0, |a| a.avg_latency);
                ScoredCandidate { index: i, score, latency }
            })
            .collect();

        // Шаг 4: Pareto front (опционально)
        if pareto_enabled && scored.len() > 1 {
            let pareto_indices = pareto_front_indices(&scored);
            let pick = pareto_indices[self.rng.gen_range(0..pareto_indices.len())];
            return Ok(&candidates[scored[pick].index]);
        }

        // Шаг 5: MaxBy (O(N))
        let best = scored
            .iter()
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
            .ok_or(AiError::NoCandidates)?;

        Ok(&candidates[best.index])
    }

    /// Лучшая известная стратегия.
    pub fn best_known<'a>(
        &self,
        candidates: &'a [StrategyGenome],
        network_hash: &str,
    ) -> Option<&'a StrategyGenome> {
        let local = self.local_bandits.get(network_hash)?;
        let mut best: Option<&'a StrategyGenome> = None;
        let mut best_mean = -1.0;

        for g in candidates {
            if let Some(arm) = local.get(&g.id.to_string()) {
                if arm.pulls() >= 1.0 {
                    let mean = arm.mean();
                    if mean > best_mean {
                        best_mean = mean;
                        best = Some(g);
                    }
                }
            }
        }
        best
    }

    pub fn register_success(&mut self, genome_id: &str) {
        self.failure_streak.remove(genome_id);
        self.blocked_until.remove(genome_id);
    }

    pub fn register_failure(&mut self, genome_id: &str, _failure_signature: Option<&str>) {
        let streak = self
            .failure_streak
            .entry(genome_id.to_string())
            .and_modify(|s| *s += 1)
            .or_insert(1);

        let backoff = backoff_ms(*streak);
        let jitter = 1.0 + (self.rng.gen::<f64>() * 0.7 - 0.35);
        let delay_ms = (backoff as f64 * jitter) as u64;
        let until = Utc::now() + chrono::Duration::milliseconds(delay_ms as i64);

        self.blocked_until
            .entry(genome_id.to_string())
            .and_modify(|existing| {
                if until > *existing {
                    *existing = until;
                }
            })
            .or_insert(until);
    }

    pub fn record_trial(
        &mut self,
        genome_id: &str,
        network_hash: &str,
        success: bool,
        latency_ms: f64,
    ) {
        let local = self.local_bandits.entry(network_hash.to_string()).or_default();
        let arm = local.entry(genome_id.to_string()).or_insert(BanditArm::new());
        arm.update(success, latency_ms);

        let global_arm = self.global_arms.entry(genome_id.to_string()).or_insert(BanditArm::new());
        global_arm.update(success, latency_ms);
    }

    pub fn total_pulls_on_network(&self, network_hash: &str) -> f64 {
        self.local_bandits
            .get(network_hash)
            .map(|m| m.values().map(|a| a.pulls()).sum())
            .unwrap_or(0.0)
    }
}

impl Default for BanditSelector {
    fn default() -> Self {
        Self::new()
    }
}

// ========== Pure functions (no self borrow) ==========

/// Pareto front indices — multi-objective (score ↑, latency ↓)
fn pareto_front_indices(scored: &[ScoredCandidate]) -> Vec<usize> {
    let mut front = Vec::new();
    for (i, item) in scored.iter().enumerate() {
        let mut dominated = false;
        for other in scored {
            if other.index == item.index {
                continue;
            }
            if other.score >= item.score && other.latency <= item.latency
                && (other.score > item.score || other.latency < item.latency)
            {
                dominated = true;
                break;
            }
        }
        if !dominated {
            front.push(i);
        }
    }
    front
}

/// Gamma sample (Marsaglia & Tsang)
fn gamma_sample(rng: &mut impl Rng, shape: f64) -> f64 {
    if shape < 1e-9 {
        return 1e-9;
    }
    if shape < 1.0 {
        return gamma_sample(rng, shape + 1.0) * rng.gen::<f64>().powf(1.0 / shape);
    }

    let d = shape - 1.0 / 3.0;
    let c = 1.0 / (9.0 * d).sqrt();

    loop {
        let mut x;
        loop {
            x = normal_sample(rng);
            if x > -1.0 / c {
                break;
            }
        }

        let v = (1.0 + c * x).powi(3);
        let u: f64 = rng.gen();
        if u < 1.0 - 0.0331 * x.powi(4) {
            return d * v;
        }
        if u.ln() < 0.5 * x * x + d * (1.0 - v + v.ln()) {
            return d * v;
        }
    }
}

/// Normal sample (Box-Muller)
fn normal_sample(rng: &mut impl Rng) -> f64 {
    let u1: f64 = 1.0 - rng.gen::<f64>();
    let u2: f64 = 1.0 - rng.gen::<f64>();
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

/// Beta sample
pub fn sample_beta(rng: &mut impl Rng, alpha: f64, beta: f64) -> f64 {
    let x = gamma_sample(rng, alpha);
    let y = gamma_sample(rng, beta);
    x / (x + y + 1e-12)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::{DpiEngineType, StrategyGenome, StrategyOrigin};

    fn make_test_genome(desync_mode: &str) -> StrategyGenome {
        let mut g = StrategyGenome::new(DpiEngineType::Zapret, StrategyOrigin::Builtin);
        g.desync_mode = desync_mode.into();
        g
    }

    #[test]
    fn test_bandit_arm_default() {
        let arm = BanditArm::new();
        assert_eq!(arm.pulls(), 0.0);
        assert_eq!(arm.mean(), 0.5);
    }

    #[test]
    fn test_bandit_arm_update_success() {
        let mut arm = BanditArm::new();
        arm.update(true, 100.0);
        assert_eq!(arm.alpha, 2.0);
        assert_eq!(arm.beta, 1.0);
        assert_eq!(arm.pulls(), 1.0);
    }

    #[test]
    fn test_bandit_arm_update_failure() {
        let mut arm = BanditArm::new();
        arm.update(false, 500.0);
        assert_eq!(arm.alpha, 1.0);
        assert_eq!(arm.beta, 2.0);
    }

    #[test]
    fn test_pick_returns_ok_with_candidates() {
        let mut selector = BanditSelector::new();
        let candidates = vec![make_test_genome("split"), make_test_genome("fake")];
        let result = selector.pick(&candidates, "test-net", 100, false, false, 0.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pick_returns_err_empty() {
        let mut selector = BanditSelector::new();
        let candidates: Vec<StrategyGenome> = vec![];
        let result = selector.pick(&candidates, "test-net", 100, false, false, 0.0);
        assert!(matches!(result, Err(AiError::NoCandidates)));
    }

    #[test]
    fn test_best_known_after_trials() {
        let mut selector = BanditSelector::new();
        let g1 = make_test_genome("split");
        let g2 = make_test_genome("fake");

        selector.record_trial(&g1.id.to_string(), "test-net", true, 100.0);
        selector.record_trial(&g1.id.to_string(), "test-net", true, 200.0);
        selector.record_trial(&g2.id.to_string(), "test-net", false, 300.0);

        let candidates = [g1, g2];
        let best = selector.best_known(&candidates, "test-net");
        assert!(best.is_some());
    }

    #[test]
    fn test_register_failure_blocks() {
        let mut selector = BanditSelector::new();
        let g = make_test_genome("split");
        let candidates = [g];
        selector.register_failure(&candidates[0].id.to_string(), None);
        // Сразу после блокировки — pick должен быть Err
        let result = selector.pick(&candidates, "test-net", 0, false, false, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_record_trial_updates_stats() {
        let mut selector = BanditSelector::new();
        let g = make_test_genome("split");

        selector.record_trial(&g.id.to_string(), "net1", true, 50.0);
        selector.record_trial(&g.id.to_string(), "net1", false, 200.0);

        assert_eq!(selector.total_pulls_on_network("net1"), 2.0);
    }

    #[test]
    fn test_pareto_front_two_strategies() {
        let scored = vec![
            ScoredCandidate { index: 0, score: 0.7, latency: 50.0 },
            ScoredCandidate { index: 1, score: 0.5, latency: 10.0 },
        ];
        let front = pareto_front_indices(&scored);
        assert_eq!(front.len(), 2);
    }

    #[test]
    fn test_pareto_front_dominated() {
        let scored = vec![
            ScoredCandidate { index: 0, score: 0.9, latency: 100.0 },
            ScoredCandidate { index: 1, score: 0.7, latency: 200.0 },
        ];
        let front = pareto_front_indices(&scored);
        assert_eq!(front.len(), 1);
    }

    #[test]
    fn test_sample_beta_range() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let s = sample_beta(&mut rng, 5.0, 2.0);
            assert!(s >= 0.0 && s <= 1.0, "sample={s}");
        }
    }
}
