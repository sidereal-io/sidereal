-- A null storage_id means storage-enabled startup has not paired a store yet.
ALTER TABLE public.sidereal_metadata ADD COLUMN storage_id UUID;
