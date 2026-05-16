use serde::{Deserialize, Serialize};

use crate::errors::CustomError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Granularity {
    Hour,
    Day,
}

impl Granularity {
    pub fn bucket_ms(self) -> i64 {
        match self {
            Granularity::Hour => 60 * 60 * 1000,
            Granularity::Day => 24 * 60 * 60 * 1000,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResultsQueryParams {
    /// Inclusive lower bound, ms since epoch. Defaults to `started_at`.
    #[serde(default)]
    pub start: Option<i64>,
    /// Inclusive upper bound, ms since epoch. Defaults to
    /// `min(now, stopped_at + 7d)`.
    #[serde(default)]
    pub end: Option<i64>,
    #[serde(default)]
    pub granularity: Option<Granularity>,
    /// Defaults to the experiment's `primary_metric`.
    #[serde(default)]
    pub metric: Option<String>,
}

impl GetResultsQueryParams {
    pub fn validate(&self) -> Result<(), CustomError> {
        if let (Some(s), Some(e)) = (self.start, self.end) {
            if s > e {
                return Err(CustomError::ValidationError(
                    "start must be <= end".into(),
                ));
            }
        }
        if let Some(name) = self.metric.as_deref() {
            if name.is_empty() {
                return Err(CustomError::ValidationError(
                    "metric must not be empty when provided".into(),
                ));
            }
            if name.len() > 256 {
                return Err(CustomError::ValidationError(
                    "metric length should be less than 256 bytes".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantResult {
    pub variant_key: String,
    pub is_control: bool,
    pub exposures: u64,
    pub converters: u64,
    /// Total count of conversion events attributed to entities in this variant
    /// (post-exposure only). For binary metrics this equals total successes;
    /// for continuous metrics this is the event count, not the value sum.
    pub total_conversions: u64,
    /// Sum of `metric_value` across attributed events, treating null as 1.0.
    pub total_value: f64,
    /// converters / exposures.
    pub conversion_rate: Option<f64>,
    /// 95% Wilson score interval for `conversion_rate`, [low, high].
    pub ci95: Option<[f64; 2]>,
    /// (p_treatment - p_control) / p_control. None on the control row and on
    /// treatment rows where the control rate is zero.
    pub lift: Option<f64>,
    /// Two-sided p-value for the treatment vs control proportion z-test.
    /// None on the control row.
    pub p_value: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SrmResult {
    pub chi_square: f64,
    pub p_value: f64,
    /// True iff p_value < 0.001 — the conventional "something is broken"
    /// threshold for sample ratio mismatch.
    pub warning: bool,
    /// Expected share per variant, in [0, 1].
    pub expected: Vec<SrmShare>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SrmShare {
    pub variant_key: String,
    pub expected: f64,
    pub actual: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeSeriesBucket {
    pub bucket_start_ms: i64,
    /// variant_key -> exposure count for this bucket.
    pub per_variant: std::collections::BTreeMap<String, u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultsResponse {
    pub experiment_id: String,
    pub experiment_key: String,
    pub metric_name: String,
    pub window_start_ms: i64,
    pub window_end_ms: i64,
    pub granularity: Granularity,
    pub variants: Vec<VariantResult>,
    pub srm: Option<SrmResult>,
    pub time_series: Vec<TimeSeriesBucket>,
}
