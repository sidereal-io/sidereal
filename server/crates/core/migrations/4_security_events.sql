-- No foreign key: username is a copy, so a row outlives its user.
CREATE TABLE public.security_events (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    occurred_at TIMESTAMPTZ NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('instance_claim', 'sign_in', 'sign_out', 'access_recovery')),
    outcome TEXT NOT NULL CHECK (outcome IN ('success', 'wrong_credentials', 'limited')),
    source TEXT NOT NULL CHECK (source IN ('http', 'siderealctl')),
    username TEXT,
    client_address INET,
    user_agent TEXT,
    sessions_ended BIGINT,
    CHECK (source = 'http' OR (client_address IS NULL AND user_agent IS NULL))
);
