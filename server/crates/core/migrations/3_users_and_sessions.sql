CREATE TABLE public.users (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role = 'admin'),
    created_at TIMESTAMPTZ NOT NULL
);
-- This release allows one administrator. The index, not application code,
-- decides a race between two first claims.
CREATE UNIQUE INDEX users_one_admin ON public.users (role) WHERE role = 'admin';

-- The browser holds the token; only its SHA-256 digest is stored.
CREATE TABLE public.sessions (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    token_hash BYTEA NOT NULL UNIQUE,
    user_id BIGINT NOT NULL REFERENCES public.users (id) ON DELETE CASCADE,
    auth_type TEXT NOT NULL CHECK (auth_type = 'password'),
    created_at TIMESTAMPTZ NOT NULL,
    last_used_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);
