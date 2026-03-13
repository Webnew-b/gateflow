-- 0001_init.sql
-- Gateflow gateway v0 schema

-- 1) apps: registry source of truth
CREATE TABLE IF NOT EXISTS apps (
  app_uuid       UUID PRIMARY KEY,
  name           TEXT NOT NULL UNIQUE,
  target_url     TEXT NOT NULL,
  status         TEXT NOT NULL,

  mount_path     TEXT NOT NULL,
  upstream_path  TEXT NOT NULL,

  app_secret     TEXT NOT NULL,

  created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- avoid ambiguous routing
CREATE UNIQUE INDEX IF NOT EXISTS apps_mount_path_uniq ON apps(mount_path);

-- minimal status machine constraint
ALTER TABLE apps
  DROP CONSTRAINT IF EXISTS apps_status_chk;
ALTER TABLE apps
  ADD CONSTRAINT apps_status_chk
  CHECK (status IN ('Registered','Active','Disabled'));

-- 2) admin users
CREATE TABLE IF NOT EXISTS admin_users (
  user_id        BIGSERIAL PRIMARY KEY,
  username       TEXT NOT NULL UNIQUE,
  password_hash  TEXT NOT NULL,
  is_active      BOOLEAN NOT NULL DEFAULT true,

  created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 3) cli sessions (token-based)
CREATE TABLE IF NOT EXISTS cli_sessions (
  session_id     BIGSERIAL PRIMARY KEY,
  user_id        BIGINT NOT NULL REFERENCES admin_users(user_id) ON DELETE CASCADE,

  session_token  TEXT NOT NULL UNIQUE,
  issued_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at     TIMESTAMPTZ NOT NULL,
  revoked_at     TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS cli_sessions_user_id_idx ON cli_sessions(user_id);
CREATE INDEX IF NOT EXISTS cli_sessions_token_idx ON cli_sessions(session_token);
CREATE INDEX IF NOT EXISTS cli_sessions_expires_idx ON cli_sessions(expires_at);
