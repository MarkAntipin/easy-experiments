use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantResult {
    pub variant_key: String,
    pub is_control: bool,
    pub exposures: u64,
    pub converters: u64,
    /// converters / exposures.
    pub conversion_rate: Option<f64>,
    /// (p_treatment - p_control) / p_control. None on the control row and on
    /// treatment rows where the control rate is zero.
    pub lift: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SrmResult {
    /// True when the observed split is significantly different from the
    /// configured one (χ² test, p < 0.001 — the conventional "something is
    /// broken" threshold for sample ratio mismatch).
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
    /// Bucket size used for the time series. The server auto-picks based on
    /// how long the experiment has been running.
    pub granularity: Granularity,
    pub variants: Vec<VariantResult>,
    pub srm: Option<SrmResult>,
    pub time_series: Vec<TimeSeriesBucket>,
}
