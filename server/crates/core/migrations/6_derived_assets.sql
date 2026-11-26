-- Whether an Asset is an original, brought in by a Source, or derived, made
-- by an Operator run (ADR-018). Every Asset that exists now is an original.
-- The default fills those rows, then goes, so every insert names its origin.
ALTER TABLE public.assets
    ADD COLUMN origin TEXT NOT NULL DEFAULT 'source'
    CHECK (origin IN ('source', 'derived'));
ALTER TABLE public.assets ALTER COLUMN origin DROP DEFAULT;

-- One row links a derived version to the version it was made from, in one
-- role, by the run that made it. `preview` is the only role so far.
CREATE TABLE public.asset_lineage (
    output_version_id UUID NOT NULL REFERENCES public.asset_versions (id),
    input_version_id UUID NOT NULL REFERENCES public.asset_versions (id),
    role TEXT NOT NULL CHECK (role IN ('preview')),
    run_id UUID NOT NULL REFERENCES public.operation_runs (id),
    PRIMARY KEY (output_version_id, input_version_id),
    CHECK (output_version_id <> input_version_id)
);
CREATE INDEX asset_lineage_input ON public.asset_lineage (input_version_id, role);
