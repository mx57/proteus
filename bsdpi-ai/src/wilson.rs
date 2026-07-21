//! Wilson Score Lower Bound — статистическая оценка качества стратегии.
//!
//! Используется для ранжирования стратегий по 95% доверительному интервалу.
//! Формула: нижняя граница Уилсона для биномиальной пропорции.
//!
//! C# оригинал: `BSDPI.AI/Math/WilsonScore.cs`

/// Wilson Score с z=1.96 (95% доверительный интервал).
pub struct WilsonScore;

impl WilsonScore {
    /// Вычисляет нижнюю границу Wilson Score (Wilson Lower Bound).
    ///
    /// # Arguments
    /// * `successes` — количество успешных попыток
    /// * `trials` — общее количество попыток
    /// * `z` — z-score (по умолчанию 1.96 для 95% CI)
    ///
    /// # Returns
    /// Значение от 0 до 1 — нижняя граница доверительного интервала.
    /// Возвращает 0 если trials <= 0.
    pub fn lower_bound(successes: u32, trials: u32, z: f64) -> f64 {
        if trials == 0 || successes > trials {
            return 0.0;
        }
        let trials_f = trials as f64;
        let phat = successes as f64 / trials_f;
        let z2 = z * z;
        let denom = 1.0 + z2 / trials_f;
        let center = phat + z2 / (2.0 * trials_f);
        let margin = z * ((phat * (1.0 - phat) + z2 / (4.0 * trials_f)) / trials_f).sqrt();
        let result = (center - margin) / denom;
        result.clamp(0.0, 1.0)
    }

    /// Среднее арифметическое для набора оценок.
    pub fn mean_score(scores: &[u32]) -> f64 {
        if scores.is_empty() {
            return 0.0;
        }
        let sum: u64 = scores.iter().map(|&x| x as u64).sum();
        sum as f64 / scores.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wilson_zero_trials() {
        let score = WilsonScore::lower_bound(0, 0, 1.96);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_wilson_all_success() {
        // 100/100 success — должно быть около 0.963
        let score = WilsonScore::lower_bound(100, 100, 1.96);
        assert!((score - 0.963).abs() < 0.01, "got {}", score);
    }

    #[test]
    fn test_wilson_all_fail() {
        // 0/100 success — должно быть около 0.0
        let score = WilsonScore::lower_bound(0, 100, 1.96);
        assert!((score - 0.0).abs() < 0.01, "got {}", score);
    }

    #[test]
    fn test_wilson_half() {
        // 50/100 — должно быть около 0.403
        let score = WilsonScore::lower_bound(50, 100, 1.96);
        assert!((score - 0.403).abs() < 0.01, "got {}", score);
    }

    #[test]
    fn test_wilson_few_trials() {
        // 3/5 с z=1.0 для стабильности
        let score = WilsonScore::lower_bound(3, 5, 1.0);
        assert!(score > 0.0 && score < 1.0, "got {}", score);
    }

    #[test]
    fn test_mean_empty() {
        let mean = WilsonScore::mean_score(&[]);
        assert_eq!(mean, 0.0);
    }

    #[test]
    fn test_mean_values() {
        let mean = WilsonScore::mean_score(&[10, 20, 30]);
        assert!((mean - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_wilson_different_z() {
        // Меньший z = более узкий интервал
        let wide = WilsonScore::lower_bound(5, 10, 3.0);
        let narrow = WilsonScore::lower_bound(5, 10, 0.5);
        // С меньшим z результат ближе к phat=0.5
        assert!((narrow - 0.5).abs() < (wide - 0.5).abs(), "narrow={} wide={}", narrow, wide);
    }

    #[test]
    fn test_wilson_output_range() {
        for s in 0..=10 {
            for t in 1..=10 {
                let score = WilsonScore::lower_bound(s, t, 1.96);
                assert!((0.0..=1.0).contains(&score), "score={} for s={} t={}", score, s, t);
            }
        }
    }
}
