CREATE TABLE api_keys (
    api_key_id    TEXT PRIMARY KEY,
    company_id    TEXT NOT NULL REFERENCES companies(company_id),
    name          TEXT NOT NULL,
    key_hash      TEXT NOT NULL UNIQUE,
    key_prefix    TEXT NOT NULL,
    created_at    INTEGER NOT NULL
);

CREATE INDEX idx_api_keys_company ON api_keys(company_id);
