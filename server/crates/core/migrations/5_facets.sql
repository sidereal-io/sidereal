-- One operation run per attempt by an Operator over one Asset version. A
-- finished run never changes: only a `running` row is ever updated.
CREATE TABLE public.operation_runs (
    id UUID PRIMARY KEY,
    operator_id TEXT NOT NULL CHECK (operator_id <> ''),
    operator_version TEXT NOT NULL CHECK (operator_version <> ''),
    asset_version_id UUID NOT NULL REFERENCES public.asset_versions (id),
    actor_user_id BIGINT NOT NULL REFERENCES public.users (id),
    status TEXT NOT NULL CHECK (status IN ('running', 'succeeded', 'failed')),
    reason TEXT CHECK (btrim(reason) <> ''),
    started_at TIMESTAMPTZ NOT NULL,
    ended_at TIMESTAMPTZ,
    CHECK ((status = 'failed') = (reason IS NOT NULL)),
    CHECK ((status = 'running') = (ended_at IS NULL)),
    CHECK (ended_at >= started_at)
);

-- One row per facet schema version, written when a value first needs it and
-- never changed, so a value always reads with its own label and unit.
-- Core checks `value_type`, so a new type needs no migration.
CREATE TABLE public.facet_schemas (
    name TEXT NOT NULL CHECK (name ~ '^[a-z0-9-]+(\.[a-z0-9-]+)+$'),
    version INTEGER NOT NULL CHECK (version >= 1),
    owner_pack TEXT NOT NULL CHECK (owner_pack <> ''),
    label TEXT NOT NULL CHECK (label <> ''),
    value_type TEXT NOT NULL CHECK (value_type <> ''),
    unit TEXT NOT NULL,
    scope TEXT NOT NULL CHECK (scope IN ('asset', 'asset_version')),
    PRIMARY KEY (name, version)
);

-- One row per stored value, with its provenance (ADR-017). A value belongs
-- to an Asset or to an Asset version, never both. Only core writes here.
CREATE TABLE public.facet_values (
    id UUID PRIMARY KEY,
    facet TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    asset_id UUID REFERENCES public.assets (id),
    asset_version_id UUID REFERENCES public.asset_versions (id),
    value JSONB NOT NULL CHECK (jsonb_typeof(value) <> 'null'),
    producer_id TEXT NOT NULL CHECK (producer_id <> ''),
    producer_version TEXT NOT NULL CHECK (producer_version <> ''),
    run_id UUID NOT NULL REFERENCES public.operation_runs (id),
    written_at TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (facet, schema_version) REFERENCES public.facet_schemas (name, version),
    CHECK ((asset_id IS NULL) <> (asset_version_id IS NULL))
);
CREATE INDEX facet_values_asset ON public.facet_values (asset_id);
CREATE INDEX facet_values_asset_version ON public.facet_values (asset_version_id);
CREATE INDEX facet_values_value ON public.facet_values USING GIN (value jsonb_path_ops);
