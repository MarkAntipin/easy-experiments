use std::collections::BTreeMap;

use duckdb::{params, Connection};

use crate::errors::CustomError;
use crate::models::{Granularity, TimeSeriesBucket};

#[derive(Debug, Clone)]
pub struct VariantAggregate {
    pub variant_key: String,
    pub exposures: u64,
    pub converters: u64,
}

/// Per-variant rollup of exposures attributed against post-exposure metric
/// events for the given window. All filters scope to `company_id` first.
pub fn variant_aggregates(
    conn: &Connection,
    company_id: &str,
    experiment_id: &str,
    metric_name: &str,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<VariantAggregate>, CustomError> {
    let sql = "
        WITH exposed AS (
            SELECT
                entity_id,
                variant_key,
                MIN(ts_ms) AS first_seen_ms
            FROM exposures
            WHERE company_id = ?
              AND experiment_id = ?
              AND ts_ms BETWEEN ? AND ?
              AND variant_key IS NOT NULL
            GROUP BY entity_id, variant_key
        ),
        attributed AS (
            SELECT DISTINCT
                e.variant_key,
                e.entity_id
            FROM exposed e
            JOIN metric_events m
              ON m.company_id  = ?
             AND m.entity_id   = e.entity_id
             AND m.metric_name = ?
             AND m.ts_ms      >= e.first_seen_ms
             AND m.ts_ms      <= ?
        )
        SELECT
            exp.variant_key,
            COUNT(DISTINCT exp.entity_id) AS exposures,
            COUNT(DISTINCT att.entity_id) AS converters
        FROM exposed exp
        LEFT JOIN attributed att
               ON att.variant_key = exp.variant_key
              AND att.entity_id   = exp.entity_id
        GROUP BY exp.variant_key
        ORDER BY exp.variant_key
    ";

    let mut stmt = conn.prepare(sql).map_err(map_duckdb_err)?;
    let rows = stmt
        .query_map(
            params![company_id, experiment_id, start_ms, end_ms, company_id, metric_name, end_ms],
            |row| {
                let variant_key: String = row.get(0)?;
                let exposures: i64 = row.get(1)?;
                let converters: i64 = row.get(2)?;
                Ok(VariantAggregate {
                    variant_key,
                    exposures: exposures.max(0) as u64,
                    converters: converters.max(0) as u64,
                })
            },
        )
        .map_err(map_duckdb_err)?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(map_duckdb_err)?);
    }
    Ok(out)
}

/// Daily/hourly exposure counts per variant in the window. Returns an ordered
/// list of buckets; each bucket has counts for any variants that received
/// traffic in that bucket. Missing variants in a bucket = zero exposures.
pub fn time_series(
    conn: &Connection,
    company_id: &str,
    experiment_id: &str,
    start_ms: i64,
    end_ms: i64,
    granularity: Granularity,
) -> Result<Vec<TimeSeriesBucket>, CustomError> {
    let bucket_ms = granularity.bucket_ms();
    let sql = "
        SELECT
            CAST(floor(ts_ms / CAST(? AS DOUBLE)) AS BIGINT) * ? AS bucket_ms,
            variant_key,
            COUNT(*) AS exposures
        FROM exposures
        WHERE company_id = ?
          AND experiment_id = ?
          AND ts_ms BETWEEN ? AND ?
          AND variant_key IS NOT NULL
        GROUP BY bucket_ms, variant_key
        ORDER BY bucket_ms ASC, variant_key ASC
    ";

    let mut stmt = conn.prepare(sql).map_err(map_duckdb_err)?;
    let rows = stmt
        .query_map(
            params![bucket_ms, bucket_ms, company_id, experiment_id, start_ms, end_ms],
            |row| {
                let bucket_ms: i64 = row.get(0)?;
                let variant_key: String = row.get(1)?;
                let count: i64 = row.get(2)?;
                Ok((bucket_ms, variant_key, count.max(0) as u64))
            },
        )
        .map_err(map_duckdb_err)?;

    let mut grouped: BTreeMap<i64, BTreeMap<String, u64>> = BTreeMap::new();
    for row in rows {
        let (bucket_ms, variant_key, count) = row.map_err(map_duckdb_err)?;
        grouped
            .entry(bucket_ms)
            .or_default()
            .insert(variant_key, count);
    }

    Ok(grouped
        .into_iter()
        .map(|(bucket_start_ms, per_variant)| TimeSeriesBucket {
            bucket_start_ms,
            per_variant,
        })
        .collect())
}

fn map_duckdb_err(e: duckdb::Error) -> CustomError {
    tracing::error!(error = %e, "analytics duckdb query failed");
    CustomError::InternalError("analytics query failed".to_string())
}
