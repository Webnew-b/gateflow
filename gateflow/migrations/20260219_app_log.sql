CREATE TABLE IF NOT EXISTS admin_op_logs (
  op_id          BIGSERIAL PRIMARY KEY,
  user_id        BIGINT NOT NULL REFERENCES admin_users(user_id) ON DELETE CASCADE,

  op_type        TEXT NOT NULL,
  app_uuid       UUID NULL REFERENCES apps(app_uuid) ON DELETE SET NULL,

  op_params      JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS admin_op_logs_user_id_idx ON admin_op_logs(user_id);
CREATE INDEX IF NOT EXISTS admin_op_logs_app_uuid_idx ON admin_op_logs(app_uuid);
CREATE INDEX IF NOT EXISTS admin_op_logs_type_idx ON admin_op_logs(op_type);
