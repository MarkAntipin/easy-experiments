CREATE TABLE companies (
    company_id  TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE users (
    user_id         TEXT PRIMARY KEY,
    company_id      TEXT NOT NULL REFERENCES companies(company_id),
    email           TEXT NOT NULL UNIQUE,
    name            TEXT,
    picture_url     TEXT,
    google_sub      TEXT NOT NULL UNIQUE,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE INDEX idx_users_company ON users(company_id);
CREATE INDEX idx_users_google_sub ON users(google_sub);

CREATE TABLE experiments (
    experiment_id    TEXT PRIMARY KEY,
    company_id       TEXT NOT NULL REFERENCES companies(company_id),
    key              TEXT NOT NULL,
    description      TEXT,

    status           TEXT NOT NULL DEFAULT 'draft'
        CHECK(status IN ('draft','running','stopped','deleted')),

    primary_metric   TEXT NOT NULL,
    variants         TEXT NOT NULL DEFAULT '[]',
    segments         TEXT NOT NULL DEFAULT '[]',

    started_at       INTEGER,
    stopped_at       INTEGER,
    created_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL,

    CHECK(json_valid(variants)),
    CHECK(json_valid(segments))
);

CREATE UNIQUE INDEX idx_experiments_company_key_active
    ON experiments(company_id, key) WHERE status != 'deleted';
CREATE INDEX idx_experiments_company_status_updated
    ON experiments(company_id, status, updated_at DESC);
