use statrs::distribution::{ChiSquared, ContinuousCDF};

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
    fn srm_perfect_split_has_p_one() {
        let (chi2, p) = srm_chi_square(&[500, 500], &[0.5, 0.5]).unwrap();
        assert!(approx(chi2, 0.0, 1e-9));
        assert!(approx(p, 1.0, 1e-9));
    }

    #[test]
    fn srm_detects_imbalance() {
        let (chi2, p) = srm_chi_square(&[600, 400], &[0.5, 0.5]).unwrap();
        assert!(approx(chi2, 40.0, 1e-6), "chi2={chi2}");
        assert!(p < 1e-9, "p={p}");
    }

    #[test]
    fn srm_three_way() {
        let (_, p) = srm_chi_square(&[33, 33, 34], &[1.0 / 3.0; 3]).unwrap();
        assert!(p > 0.95, "p={p}");
    }

    #[test]
    fn srm_returns_none_for_mismatched_inputs() {
        assert!(srm_chi_square(&[1, 2], &[0.5]).is_none());
        assert!(srm_chi_square(&[], &[]).is_none());
        assert!(srm_chi_square(&[0, 0], &[0.5, 0.5]).is_none());
        assert!(srm_chi_square(&[1, 1], &[0.0, 1.0]).is_none());
    }
}
