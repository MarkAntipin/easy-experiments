use statrs::distribution::{ChiSquared, ContinuousCDF, Normal};

/// Wilson 95% score interval for a binomial proportion.
///
/// Returns `[low, high]` clamped to [0, 1]. We use Wilson over the normal
/// approximation because Wilson behaves correctly at small samples and at
/// extreme rates (p near 0 or 1) — the textbook normal CI famously gives
/// negative lower bounds for `p < 1.96 * sqrt(p(1-p)/n)`.
pub fn wilson_95(successes: u64, trials: u64) -> Option<[f64; 2]> {
    if trials == 0 {
        return None;
    }
    let n = trials as f64;
    let p_hat = successes as f64 / n;
    // 1.96 = z_{0.975}; hard-coded since we only ship 95%.
    let z: f64 = 1.959_963_984_540_054;
    let z2 = z * z;
    let denom = 1.0 + z2 / n;
    let center = (p_hat + z2 / (2.0 * n)) / denom;
    let half = z * ((p_hat * (1.0 - p_hat) / n) + (z2 / (4.0 * n * n))).sqrt() / denom;
    let lo = (center - half).clamp(0.0, 1.0);
    let hi = (center + half).clamp(0.0, 1.0);
    Some([lo, hi])
}

/// Two-sided two-proportion z-test, pooled standard error.
///
/// Compares treatment proportion `p_t = s_t/n_t` to control `p_c = s_c/n_c`.
/// Returns the two-sided p-value, or `None` if either arm has zero exposures
/// or the pooled SE is zero (e.g. both arms 0% or both 100%).
pub fn two_proportion_ztest(
    control_successes: u64,
    control_trials: u64,
    treatment_successes: u64,
    treatment_trials: u64,
) -> Option<f64> {
    if control_trials == 0 || treatment_trials == 0 {
        return None;
    }
    let n_c = control_trials as f64;
    let n_t = treatment_trials as f64;
    let p_c = control_successes as f64 / n_c;
    let p_t = treatment_successes as f64 / n_t;
    let p_pool = (control_successes + treatment_successes) as f64 / (n_c + n_t);
    let se = (p_pool * (1.0 - p_pool) * (1.0 / n_c + 1.0 / n_t)).sqrt();
    if se == 0.0 || !se.is_finite() {
        return None;
    }
    let z = (p_t - p_c) / se;
    let normal = Normal::new(0.0, 1.0).ok()?;
    // Two-sided: P(|Z| >= |z|) = 2 * (1 - Phi(|z|)).
    let two_sided = 2.0 * (1.0 - normal.cdf(z.abs()));
    Some(two_sided.clamp(0.0, 1.0))
}

/// Pearson chi-square goodness-of-fit for sample ratio mismatch.
///
/// `observed[i]` is the observed exposure count in variant i; `expected[i]`
/// is the expected fraction (sum to 1.0). Returns `(chi_square, p_value)` or
/// None if inputs are invalid (mismatched lengths, all-zero observed, any
/// non-positive expected fraction).
pub fn srm_chi_square(observed: &[u64], expected: &[f64]) -> Option<(f64, f64)> {
    if observed.len() != expected.len() || observed.is_empty() {
        return None;
    }
    let total: u64 = observed.iter().sum();
    if total == 0 {
        return None;
    }
    let total_f = total as f64;

    let mut chi2 = 0.0;
    for (o, &e_frac) in observed.iter().zip(expected.iter()) {
        if e_frac <= 0.0 || !e_frac.is_finite() {
            return None;
        }
        let e = e_frac * total_f;
        let diff = *o as f64 - e;
        chi2 += diff * diff / e;
    }

    let df = (observed.len() - 1).max(1) as f64;
    let dist = ChiSquared::new(df).ok()?;
    // P(X^2 >= chi2) under H0 = 1 - CDF.
    let p_value = (1.0 - dist.cdf(chi2)).clamp(0.0, 1.0);
    Some((chi2, p_value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn wilson_handles_zero_trials() {
        assert!(wilson_95(0, 0).is_none());
    }

    #[test]
    fn wilson_clamps_to_unit_interval() {
        // 0/100 should give [~0, ~0.037]. Lower bound may not be *exactly* 0
        // due to FP rounding (it's `(c - h)` where c == h to within ulps).
        let [lo, hi] = wilson_95(0, 100).unwrap();
        assert!(lo.abs() < 1e-12, "lo={lo}");
        assert!(hi > 0.0 && hi < 0.1, "hi={hi}");

        // 100/100 should give [~0.96, ~1.0]
        let [lo, hi] = wilson_95(100, 100).unwrap();
        assert!(lo > 0.9 && lo < 1.0, "lo={lo}");
        assert!((1.0 - hi).abs() < 1e-12, "hi={hi}");
    }

    #[test]
    fn wilson_known_value() {
        // 50/100 — Wilson 95% ≈ [0.404, 0.596]
        let [lo, hi] = wilson_95(50, 100).unwrap();
        assert!(approx(lo, 0.404, 0.005), "lo={lo}");
        assert!(approx(hi, 0.596, 0.005), "hi={hi}");
    }

    #[test]
    fn ztest_no_difference_yields_high_p() {
        let p = two_proportion_ztest(50, 100, 50, 100).unwrap();
        // Identical proportions => z = 0 => p-value = 1.
        assert!(approx(p, 1.0, 1e-9), "p={p}");
    }

    #[test]
    fn ztest_clear_difference_is_significant() {
        // 200/1000 vs 250/1000 — z ≈ -2.78, two-sided p ≈ 0.0054
        let p = two_proportion_ztest(200, 1000, 250, 1000).unwrap();
        assert!(p > 0.001 && p < 0.02, "p={p}");
    }

    #[test]
    fn ztest_returns_none_on_zero_trials() {
        assert!(two_proportion_ztest(0, 0, 1, 100).is_none());
        assert!(two_proportion_ztest(1, 100, 0, 0).is_none());
    }

    #[test]
    fn ztest_returns_none_when_se_is_zero() {
        // Both arms 0% — pooled p = 0, SE = 0.
        assert!(two_proportion_ztest(0, 100, 0, 100).is_none());
        // Both arms 100% — pooled p = 1, SE = 0.
        assert!(two_proportion_ztest(100, 100, 100, 100).is_none());
    }

    #[test]
    fn srm_perfect_split_has_p_one() {
        let (chi2, p) = srm_chi_square(&[500, 500], &[0.5, 0.5]).unwrap();
        assert!(approx(chi2, 0.0, 1e-9));
        assert!(approx(p, 1.0, 1e-9));
    }

    #[test]
    fn srm_detects_imbalance() {
        // 600 vs 400 against expected 50/50 — chi2 = 40, p tiny.
        let (chi2, p) = srm_chi_square(&[600, 400], &[0.5, 0.5]).unwrap();
        assert!(approx(chi2, 40.0, 1e-6), "chi2={chi2}");
        assert!(p < 1e-9, "p={p}");
    }

    #[test]
    fn srm_three_way() {
        // 33/33/34 vs 1/3 — should be near-perfect fit.
        let (_, p) = srm_chi_square(&[33, 33, 34], &[1.0 / 3.0; 3]).unwrap();
        assert!(p > 0.95, "p={p}");
    }

    #[test]
    fn srm_returns_none_for_mismatched_inputs() {
        assert!(srm_chi_square(&[1, 2], &[0.5]).is_none());
        assert!(srm_chi_square(&[], &[]).is_none());
        assert!(srm_chi_square(&[0, 0], &[0.5, 0.5]).is_none());
        // Non-positive expected fraction.
        assert!(srm_chi_square(&[1, 1], &[0.0, 1.0]).is_none());
    }
}
