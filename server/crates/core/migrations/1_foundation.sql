CREATE TABLE public.sidereal_metadata (
    product TEXT PRIMARY KEY CHECK (product = 'sidereal'),
    format_version INTEGER NOT NULL CHECK (format_version = 1)
);
INSERT INTO public.sidereal_metadata (product, format_version) VALUES ('sidereal', 1);
