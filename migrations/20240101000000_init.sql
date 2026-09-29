CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    full_name       TEXT NOT NULL,
    email           TEXT NOT NULL UNIQUE,
    hashed_password TEXT NOT NULL,
    role            TEXT NOT NULL CHECK (role IN ('admin', 'staff')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE tasks (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title          TEXT NOT NULL,
    description    TEXT NOT NULL DEFAULT '',
    status         TEXT NOT NULL CHECK (status IN ('todo', 'in_progress', 'done')),
    priority       TEXT NOT NULL CHECK (priority IN ('low', 'medium', 'high')),
    created_by_id  UUID NOT NULL REFERENCES users(id),
    assigned_to_id UUID NULL REFERENCES users(id),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX tasks_assigned_to_idx ON tasks (assigned_to_id, created_at, id);

CREATE TABLE login_challenges (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id),
    code_hash  TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at    TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Local development inbox. This is the only table that stores a raw code.
CREATE TABLE email_logs (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    to_email          TEXT NOT NULL,
    subject           TEXT NOT NULL,
    body              TEXT NOT NULL,
    verification_code TEXT NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX email_logs_created_at_idx ON email_logs (created_at DESC);
