-- A choice facet keeps its allowed values with its schema version (ADR-018),
-- in their declared order, so a stored choice is always read against the
-- list it was checked against. Every other type holds NULL.
ALTER TABLE public.facet_schemas
    ADD COLUMN allowed_values TEXT[],
    ADD CONSTRAINT facet_schemas_allowed_values_check CHECK (
        CASE WHEN value_type = 'choice'
            THEN allowed_values IS NOT NULL AND cardinality(allowed_values) >= 1
            ELSE allowed_values IS NULL
        END
    );
