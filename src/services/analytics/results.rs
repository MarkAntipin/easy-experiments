use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use moka::future::Cache;

use crate::errors::CustomError;
use crate::models::{
    Granularity, ResultsResponse, Segment, SrmResult, SrmShare, TimeSeriesBucket, Variant,
    VariantResult,
};
use crate::services::analytics::pool::DuckDBReadPool;
use crate::services::analytics::queries::{time_series, variant_aggregates, VariantAggregate};
use crate::services::analytics::stats::srm_chi_square;
use crate::services::experiment::get_experiment;

const ANALYTICS_QUERY_TIMEOUT: Duration = Duration::from_secs(10);

const DEFAULT_POST_STOP_OVERHANG_MS: i64 = 7 * 24 * 60 * 60 * 1000;

const END_MS_QUANTUM_MS: i64 = 60 * 1000;

const HOUR_GRANULARITY_THRESHOLD_MS: i64 = 24 * 60 * 60 * 1000;

/// (company_id, experiment_id, start_ms, end_ms)
type CacheKey = (String, String, i64, i64);

pub type ResultsCache = Cache<CacheKey, Arc<ResultsResponse>>;

pub struct ResultsService {
    pool: Arc<DuckDBReadPool>,
    cache: ResultsCache,
}

impl ResultsService {
    pub fn new(pool: Arc<DuckDBReadPool>, cache_capacity: u64, cache_ttl: Duration) -> Self {
        let cache: ResultsCache = Cache::builder()
            .max_capacity(cache_capacity)
            .time_to_live(cache_ttl)
            .build();
        Self { pool, cache }
    }

    pub async fn get_results(
        &self,
        db: &crate::models::ExperimentsDB,
        company_id: &str,
        experiment_id: &str,
    ) -> Result<Arc<ResultsResponse>, CustomError> {
        let row = get_experiment(db, experiment_id, company_id).await?;
        let resolved = resolve_window(&row);

        let cache_key: CacheKey = (
            company_id.to_string(),
            experiment_id.to_string(),
            resolved.start_ms,
            resolved.end_ms,
        );

        let pool = Arc::clone(&self.pool);
        let row = Arc::new(row);
        let row_for_loader = Arc::clone(&row);

        // single-flight via try_get_with: concurrent dashboards collapse to one query.
        let value = self
            .cache
            .try_get_with::<_, CustomError>(cache_key, async move {
                let response = compute_results(pool, &row_for_loader, resolved).await?;
                Ok(Arc::new(response))
            })
            .await
            .map_err(|arc_err| arc_err.as_ref().clone())?;

        Ok(value)
    }
}

#[derive(Debug, Clone)]
struct ResolvedWindow {
    start_ms: i64,
    end_ms: i64,
    granularity: Granularity,
    metric_name: String,
}

fn resolve_window(row: &crate::models::ExperimentRow) -> ResolvedWindow {
    let now_ms = Utc::now().timestamp_millis();

    let start_ms = row.started_at.unwrap_or(row.created_at);

    let raw_end_ms = match row.stopped_at {
        Some(stopped) => (stopped + DEFAULT_POST_STOP_OVERHANG_MS).min(now_ms),
        None => now_ms,
    };
    let end_ms = (raw_end_ms / END_MS_QUANTUM_MS) * END_MS_QUANTUM_MS;
    let end_ms = end_ms.max(start_ms);

    let granularity = if end_ms - start_ms <= HOUR_GRANULARITY_THRESHOLD_MS {
        Granularity::Hour
    } else {
        Granularity::Day
    };

    ResolvedWindow {
        start_ms,
        end_ms,
        granularity,
        metric_name: row.primary_metric.clone(),
    }
}

async fn compute_results(
    pool: Arc<DuckDBReadPool>,
    row: &crate::models::ExperimentRow,
    resolved: ResolvedWindow,
) -> Result<ResultsResponse, CustomError> {
    let variants: Vec<Variant> = serde_json::from_str(&row.variants).map_err(|e| {
        CustomError::InternalError(format!("failed to parse stored variants: {}", e))
    })?;
    let segments: Vec<Segment> = serde_json::from_str(&row.segments).map_err(|e| {
        CustomError::InternalError(format!("failed to parse stored segments: {}", e))
    })?;

    let conn_guard = pool.acquire().await?;

    // DuckDB queries are sync; run on a blocking thread, capped by timeout.
    let company_id = row.company_id.clone();
    let experiment_id = row.experiment_id.clone();
    let metric_name = resolved.metric_name.clone();
    let start_ms = resolved.start_ms;
    let end_ms = resolved.end_ms;
    let granularity = resolved.granularity;

    let join_fut = tokio::task::spawn_blocking(move || -> Result<_, CustomError> {
        let conn = conn_guard.conn();
        let aggs = variant_aggregates(
            conn,
            &company_id,
            &experiment_id,
            &metric_name,
            start_ms,
            end_ms,
        )?;
        let series = time_series(
            conn,
            &company_id,
            &experiment_id,
            start_ms,
            end_ms,
            granularity,
        )?;
        Ok((aggs, series))
    });

    let (aggs, series) = match tokio::time::timeout(ANALYTICS_QUERY_TIMEOUT, join_fut).await {
        Ok(Ok(Ok(v))) => v,
        Ok(Ok(Err(e))) => return Err(e),
        Ok(Err(join_err)) => {
            tracing::error!(error = %join_err, "analytics: spawn_blocking join failed");
            return Err(CustomError::InternalError(
                "analytics query failed".to_string(),
            ));
        }
        Err(_) => {
            tracing::error!(
                experiment_id = %row.experiment_id,
                timeout_ms = ANALYTICS_QUERY_TIMEOUT.as_millis() as u64,
                "analytics: query timed out",
            );
            return Err(CustomError::InternalError(
                "analytics query timed out".to_string(),
            ));
        }
    };

    let response = build_response(row, &variants, &segments, granularity, aggs, series);
    Ok(response)
}

fn build_response(
    row: &crate::models::ExperimentRow,
    variants: &[Variant],
    segments: &[Segment],
    granularity: Granularity,
    aggs: Vec<VariantAggregate>,
    series: Vec<TimeSeriesBucket>,
) -> ResultsResponse {
    let agg_by_variant: HashMap<String, VariantAggregate> = aggs
        .into_iter()
        .map(|a| (a.variant_key.clone(), a))
        .collect();

    let control_key = variants
        .iter()
        .find(|v| v.is_control)
        .map(|v| v.key.clone());

    // Compute control rate up-front for lift/ztest math.
    let control_stats: Option<(u64, u64)> = control_key
        .as_deref()
        .and_then(|k| agg_by_variant.get(k))
        .map(|a| (a.converters, a.exposures));

    let mut variant_results: Vec<VariantResult> = variants
        .iter()
        .map(|v| {
            let agg = agg_by_variant.get(&v.key);
            let exposures = agg.map(|a| a.exposures).unwrap_or(0);
            let converters = agg.map(|a| a.converters).unwrap_or(0);
            let conversion_rate = if exposures > 0 {
                Some(converters as f64 / exposures as f64)
            } else {
                None
            };

            let lift = if v.is_control {
                None
            } else if let (Some((c_succ, c_n)), true) = (control_stats, exposures > 0) {
                let p_c = if c_n > 0 {
                    c_succ as f64 / c_n as f64
                } else {
                    0.0
                };
                let p_t = converters as f64 / exposures as f64;
                if p_c > 0.0 {
                    Some((p_t - p_c) / p_c)
                } else {
                    None
                }
            } else {
                None
            };

            VariantResult {
                variant_key: v.key.clone(),
                is_control: v.is_control,
                exposures,
                converters,
                conversion_rate,
                lift,
            }
        })
        .collect();

    // Stable sort: control first, then by variant key.
    variant_results.sort_by(|a, b| match (a.is_control, b.is_control) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.variant_key.cmp(&b.variant_key),
    });

    let srm = compute_srm(variants, segments, &variant_results);

    ResultsResponse {
        experiment_id: row.experiment_id.clone(),
        experiment_key: row.key.clone(),
        granularity,
        variants: variant_results,
        srm,
        time_series: series,
    }
}

fn compute_srm(
    variants: &[Variant],
    segments: &[Segment],
    variant_results: &[VariantResult],
) -> Option<SrmResult> {
    // First (lowest priority) segment defines the expected distribution. We
    // sort segments by priority at evaluate-cache load time, but raw segments
    // here aren't sorted — pick min priority.
    let first_segment = segments.iter().min_by_key(|s| s.priority)?;
    let total_pct: u32 = first_segment.distributions.iter().map(|d| d.percent).sum();
    if total_pct == 0 {
        return None;
    }
    let total_pct_f = total_pct as f64;

    let expected_share: HashMap<&str, f64> = first_segment
        .distributions
        .iter()
        .map(|d| (d.variant_key.as_str(), d.percent as f64 / total_pct_f))
        .collect();

    // Order observed by the same ordering as variants (for stable output).
    let order: Vec<&str> = variants.iter().map(|v| v.key.as_str()).collect();
    let result_by_key: HashMap<&str, &VariantResult> = variant_results
        .iter()
        .map(|v| (v.variant_key.as_str(), v))
        .collect();

    let mut observed: Vec<u64> = Vec::with_capacity(order.len());
    let mut expected_fracs: Vec<f64> = Vec::with_capacity(order.len());
    let mut shares: Vec<SrmShare> = Vec::with_capacity(order.len());

    let total_exposures: u64 = result_by_key.values().map(|v| v.exposures).sum();

    for key in &order {
        let exp_share = match expected_share.get(key) {
            Some(&v) if v > 0.0 => v,
            _ => continue,
        };
        let obs = result_by_key.get(key).map(|v| v.exposures).unwrap_or(0);
        let actual = if total_exposures > 0 {
            obs as f64 / total_exposures as f64
        } else {
            0.0
        };
        observed.push(obs);
        expected_fracs.push(exp_share);
        shares.push(SrmShare {
            variant_key: (*key).to_string(),
            expected: exp_share,
            actual,
        });
    }

    let (_chi_square, p_value) = srm_chi_square(&observed, &expected_fracs)?;
    Some(SrmResult {
        warning: p_value < 0.001,
        expected: shares,
    })
}
