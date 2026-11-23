# Managed storage glossary

Terms whose meaning is narrower than, or not in, the shared
[glossary](../../glossary.md). Agree a term with the user before adding it.

- **Staging file** — a file in `tmp/` that holds an upload's bytes while ingest checks
  them, before it becomes a stored object. One left behind by a crash is removed at the
  next startup.
