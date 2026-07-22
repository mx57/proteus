//! Wilson Score Lower Bound — статистически строгая оценка качества стратегий.
//!
//! Формула Wilson Score (95% доверительный интервал) используется для ранжирования
//! стратегий по нижней границе доверительного интервала — более надёжная метрика,
//! чем просто процент успеха, особенно при малом числе испытаний.
//!
//! ## C# оригинал
//! `BSDPI.AI/Math/WilsonScore.cs`

/// Z-value для 95% доверительного интервала (стандартный)
pub const Z_95: f64 = 1.96;

/// Z-value для 90% доверительного интервала
pub const Z_90: f64 = 1.645;

/// Z-value для 99% доверительного интервала
pub const Z_99: f64 = 2.576;

/// Вычисляет **нижнюю границу Wilson Score** для биномиальной пропорции
/// с заданным Z-score (по умолчанию 1.96 = 95% CI).
///
/// **Формула:**
/// ```text
/// p̂ = successes / trials
/// z² = z * z
/// denominator = 1 + z² / trials
/// center = p̂ + z² / (2 * trials)
/// margin = z * sqrt((p̂ * (1 - p̂) + z² / (4 * trials)) / trials)
/// result = clamp((center - margin) / denominator, 0, 1)
/// ```
///
/// # Panics
/// Не паникует — возвращает 0 при `trials <= 0`.
#[inline]
pub fn lower_bound(successes: u64, trials: u64, z: f64) -> f64 {
    if trials == 0 {
        return 0.0;
    }

    let p_hat = successes as f64 / trials as f64;
    let z2 = z * z;
    let denom = 1.0 + z2 / trials as f64;
    let center = p_hat + z2 / (2.0 * trials as f64);
    let margin = z * ((p_hat * (1.0 - p_hat) + z2 / (4.0 * trials as f64)) / trials as f64).sqrt();

    ((center - margin) / denom).clamp(0.0, 1.0)
}

/// Вычисляет среднее арифметическое для списка оценок.
/// Возвращает 0 для пустого списка.
#[inline]
pub fn mean_score(scores: &[f64]) -> f64 {
    if scores.is_empty() {
        return 0.0;
    }
    scores.iter().sum::<f64>() / scores.len() as f64
}

/// Wilson Score с 95% CI (сокращение для стандартного случая).
#[inline]
pub fn lower_bound_95(successes: u64, trials: u64) -> f64 {
    lower_bound(successes, trials, Z_95)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_trials_returns_zero() {
        assert_eq!(lower_bound(0, 0, Z_95), 0.0);
        assert_eq!(lower_bound(5, 0, Z_95), 0.0);
    }

    #[test]
    fn test_all_successes_high_score() {
        // 10/10 successes → near 0.72 (Wilson correction pulls toward 0.5 for small n)
        let score = lower_bound(10, 10, Z_95);
        assert!(score > 0.65 && score < 0.80, "score = {score}");
    }

    #[test]
    fn test_all_failures_low_score() {
        // 0/10 failures → near 0.0 (but not exactly 0 because of Wilson correction)
        let score = lower_bound(0, 10, Z_95);
        assert!(score >= 0.0 && score < 0.1, "score = {score}");
    }

    #[test]
    fn test_partial_success() {
        // 5/10 = 50% → Wilson should give ~0.24
        let score = lower_bound(5, 10, Z_95);
        assert!(score > 0.18 && score < 0.30, "score = {score}");
    }

    #[test]
    fn test_more_trials_more_confidence() {
        // 50/100 vs 5/10 (both 50%) — more trials = narrower CI = higher lower bound
        let a = lower_bound(5, 10, Z_95);
        let b = lower_bound(50, 100, Z_95);
        assert!(
            b > a,
            "more trials should give higher Wilson score: {b} > {a}"
        );
    }

    #[test]
    fn test_z90_is_lower_than_z95() {
        let z95 = lower_bound(5, 10, Z_95);
        let z90 = lower_bound(5, 10, Z_90);
        assert!(z90 > z95, "90% CI should be tighter: {z90} > {z95}");
    }

    #[test]
    fn test_mean_score_empty() {
        assert_eq!(mean_score(&[]), 0.0);
    }

    #[test]
    fn test_mean_score_single() {
        assert!((mean_score(&[42.0]) - 42.0).abs() < 1e-12);
    }

    #[test]
    fn test_mean_score_multiple() {
        let scores = vec![10.0, 20.0, 30.0];
        assert!((mean_score(&scores) - 20.0).abs() < 1e-12);
    }

    #[test]
    fn test_lower_bound_clamp() {
        // Even with extreme cases, result should be in [0, 1]
        let score = lower_bound(0, 1, Z_95);
        assert!(score >= 0.0 && score <= 1.0, "score = {score}");

        let score = lower_bound(1, 1, Z_95);
        assert!(score >= 0.0 && score <= 1.0, "score = {score}");
    }
}
