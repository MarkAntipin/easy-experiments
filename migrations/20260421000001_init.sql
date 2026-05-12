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
    -- Identity proofs. At least one of `google_sub` or `password_hash` is
    -- set for an *active* user. A row with both NULL is a pending invite
    -- waiting to be claimed by either Google sign-in or `accept-invite`.
    google_sub      TEXT UNIQUE,
    password_hash   TEXT,
    -- Pending password invites carry a hashed one-time token + expiry. The
    -- plaintext is returned to the inviting admin once and never stored.
    invite_token_hash       TEXT UNIQUE,
    invite_token_expires_at INTEGER,
    role            TEXT NOT NULL DEFAULT 'member'
        CHECK(role IN ('admin','member')),
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE INDEX idx_users_company ON users(company_id);
CREATE INDEX idx_users_google_sub ON users(google_sub);
CREATE INDEX idx_users_invite_token_hash ON users(invite_token_hash);

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

CREATE TABLE api_keys (
    api_key_id    TEXT PRIMARY KEY,
    company_id    TEXT NOT NULL REFERENCES companies(company_id),
    name          TEXT NOT NULL,
    key_hash      TEXT NOT NULL UNIQUE,
    key_prefix    TEXT NOT NULL,
    created_at    INTEGER NOT NULL
);

CREATE INDEX idx_api_keys_company ON api_keys(company_id);
CREATE UNIQUE INDEX idx_api_keys_company_name ON api_keys(company_id, name);
