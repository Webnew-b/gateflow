CREATE TABLE IF NOT EXISTS app_health_latest (
  app_uuid         UUID PRIMARY KEY REFERENCES apps(app_uuid) ON DELETE CASCADE,
  last_checked_at  TIMESTAMPTZ NOT NULL,
  ok               BOOLEAN NOT NULL,
  status_code      INTEGER NOT NULL,
  latency_ms       INTEGER NOT NULL,
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS app_health_latest_updated_idx ON app_health_latest(updated_at);
