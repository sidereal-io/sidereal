-- An Asset is the stable identity; an Asset version is one immutable set of
-- bytes. Core mints every id; the content hash is neither an id nor unique.
CREATE TABLE public.assets (
    id UUID PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind <> ''),
    name TEXT NOT NULL CHECK (name <> ''),
    current_version_id UUID NOT NULL,
    revision BIGINT NOT NULL DEFAULT 0 CHECK (revision >= 0)
);

CREATE TABLE public.asset_versions (
    id UUID PRIMARY KEY,
    asset_id UUID NOT NULL REFERENCES public.assets (id),
    content_hash TEXT NOT NULL CHECK (content_hash ~ '^[0-9a-f]{64}$'),
    byte_size BIGINT NOT NULL CHECK (byte_size >= 0),
    media_type TEXT NOT NULL CHECK (media_type <> ''),
    source_id TEXT NOT NULL CHECK (source_id <> ''),
    source_version TEXT NOT NULL CHECK (source_version <> ''),
    ingested_at TIMESTAMPTZ NOT NULL,
    UNIQUE (asset_id, id)
);
CREATE INDEX asset_versions_content_hash ON public.asset_versions (content_hash);

-- The current version must exist and belong to the same Asset. The check
-- waits for commit, so an Asset and its first version go in one transaction.
ALTER TABLE public.assets ADD CONSTRAINT assets_current_version_fkey
    FOREIGN KEY (id, current_version_id) REFERENCES public.asset_versions (asset_id, id)
    DEFERRABLE INITIALLY DEFERRED;
