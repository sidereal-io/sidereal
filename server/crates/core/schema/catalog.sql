-- Stable logical schema: no owners, privileges, physical IDs, or timestamps.
SELECT jsonb_build_object(
 'schemas', (SELECT coalesce(jsonb_agg(nspname ORDER BY nspname), '[]') FROM pg_namespace WHERE nspname !~ '^pg_' AND nspname <> 'information_schema'),
 'relations', (SELECT coalesce(jsonb_agg(jsonb_build_object(
   'schema', n.nspname, 'name', c.relname, 'kind', c.relkind::text,
   'columns', (SELECT coalesce(jsonb_agg(jsonb_build_object(
      'name', a.attname, 'type', format_type(a.atttypid,a.atttypmod), 'nullable', NOT a.attnotnull,
      'default', pg_get_expr(d.adbin,d.adrelid), 'generated', a.attgenerated::text, 'identity', a.attidentity::text
    ) ORDER BY a.attnum), '[]') FROM pg_attribute a LEFT JOIN pg_attrdef d ON d.adrelid=a.attrelid AND d.adnum=a.attnum WHERE a.attrelid=c.oid AND a.attnum>0 AND NOT a.attisdropped),
   'constraints', (SELECT coalesce(jsonb_agg(pg_get_constraintdef(k.oid,true) ORDER BY pg_get_constraintdef(k.oid,true)), '[]') FROM pg_constraint k WHERE k.conrelid=c.oid),
   'indexes', (SELECT coalesce(jsonb_agg(jsonb_build_object('definition', pg_get_indexdef(i.indexrelid), 'valid', i.indisvalid, 'ready', i.indisready) ORDER BY pg_get_indexdef(i.indexrelid)), '[]') FROM pg_index i WHERE i.indrelid=c.oid),
   'view', CASE WHEN c.relkind IN ('v','m') THEN pg_get_viewdef(c.oid,true) ELSE NULL END,
   'rls', c.relrowsecurity, 'force_rls', c.relforcerowsecurity
 ) ORDER BY n.nspname,c.relname), '[]') FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname !~ '^pg_' AND n.nspname <> 'information_schema'),
 'types', (SELECT coalesce(jsonb_agg(jsonb_build_object('schema',n.nspname,'name',t.typname,'kind',t.typtype::text,'base',format_type(t.typbasetype,t.typtypmod),'not_null',t.typnotnull,'default',t.typdefault,'enum', (SELECT coalesce(jsonb_agg(e.enumlabel ORDER BY e.enumsortorder),'[]') FROM pg_enum e WHERE e.enumtypid=t.oid)) ORDER BY n.nspname,t.typname),'[]') FROM pg_type t JOIN pg_namespace n ON n.oid=t.typnamespace WHERE n.nspname !~ '^pg_' AND n.nspname <> 'information_schema'),
 'functions', (SELECT coalesce(jsonb_agg(jsonb_build_object('schema',n.nspname,'name',p.proname,'definition',pg_get_functiondef(p.oid)) ORDER BY n.nspname,p.proname,p.oid),'[]') FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname !~ '^pg_' AND n.nspname <> 'information_schema'),
 'triggers', (SELECT coalesce(jsonb_agg(pg_get_triggerdef(t.oid,true) ORDER BY pg_get_triggerdef(t.oid,true)), '[]') FROM pg_trigger t WHERE NOT t.tgisinternal),
 'extensions', (SELECT coalesce(jsonb_agg(extname ORDER BY extname),'[]') FROM pg_extension WHERE extname <> 'plpgsql')
)::text
