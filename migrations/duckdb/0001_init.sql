CREATE TABLE IF NOT EXISTS exposures (
    schema_version  INTEGER NOT NULL,
    ts_ms           BIGINT  NOT NULL,
    company_id      VARCHAR NOT NULL,
    experiment_id   VARCHAR NOT NULL,
    variant_key     VARCHAR,
    entity_id       VARCHAR NOT NULL
);

CREATE TABLE IF NOT EXISTS metric_events (
    schema_version  INTEGER NOT NULL,
    ts_ms           BIGINT  NOT NULL,
    company_id      VARCHAR NOT NULL,
    entity_id       VARCHAR NOT NULL,
    metric_name     VARCHAR NOT NULL,
    metric_value    DOUBLE
);
