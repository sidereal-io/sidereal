#!/usr/bin/env node
/**
 * Database Migration Script — PostgreSQL ↔ SQLite
 *
 * Migrates all data between PostgreSQL and SQLite in either direction.
 * Discovers tables and column types dynamically from database metadata —
 * no manual updates needed when the schema changes.
 *
 * Usage:
 *   # PostgreSQL → SQLite
 *   node tools/scripts/migrate-db.js --from postgresql://user:pass@host:5432/sidereal --to sqlite:path/to/sidereal.db
 *
 *   # SQLite → PostgreSQL
 *   node tools/scripts/migrate-db.js --from sqlite:path/to/local.db --to postgresql://user:pass@host:5432/sidereal
 *
 * Inside Docker container:
 *   node /app/dist/tools/scripts/migrate-db.js --from postgresql://... --to sqlite:/app/config/sidereal.db
 */

import postgres from 'postgres';
import Database from 'better-sqlite3';
import { drizzle as sqliteDrizzle } from 'drizzle-orm/better-sqlite3';
import { migrate as sqliteMigrate } from 'drizzle-orm/better-sqlite3/migrator';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const SQLITE_MIGRATIONS_PATH = path.resolve(__dirname, '../migrations/sqlite');

// Internal tables to skip during migration
const SKIP_TABLES = new Set([
  '__drizzle_migrations',
  'drizzle_migrations',
  'sqlite_sequence',
]);

/**
 * @typedef {Object} ColumnInfo
 * @property {string} name
 * @property {string} type // normalized: 'text', 'integer', 'real', 'boolean', 'timestamp', 'json', 'text[]'
 */

/**
 * @typedef {Object} TableInfo
 * @property {string} name
 * @property {ColumnInfo[]} columns
 * @property {string[]} dependsOn Tables this table references via foreign keys
 */

/**
 * @typedef {Object} DbConnection
 * @property {'postgresql'|'sqlite'} type
 * @property {() => Promise<TableInfo[]>} introspect Discover all user tables, their columns, and FK dependencies
 * @property {(table: string) => Promise<object[]>} readAll
 * @property {(table: string, rows: object[], columns: ColumnInfo[]) => Promise<number>} insertRows
 * @property {() => Promise<void>} close
 */

function parseConnectionString(connStr) {
  if (connStr.startsWith('postgresql://') || connStr.startsWith('postgres://')) {
    return { type: 'postgresql', url: connStr };
  }
  if (connStr.startsWith('sqlite:')) {
    return { type: 'sqlite', url: connStr.slice('sqlite:'.length) };
  }
  throw new Error(`Unknown connection string format: ${connStr}. Use postgresql://... or sqlite:path/to/file.db`);
}

/** Sort tables so dependencies come before dependents */
function topologicalSort(tables) {
  const byName = new Map(tables.map(t => [t.name, t]));
  const sorted = [];
  const visited = new Set();
  const visiting = new Set();

  function visit(name) {
    if (visited.has(name)) return;
    if (visiting.has(name)) return; // circular — break the cycle
    visiting.add(name);
    const table = byName.get(name);
    if (table) {
      for (const dep of table.dependsOn) {
        if (byName.has(dep)) visit(dep);
      }
      sorted.push(table);
    }
    visiting.delete(name);
    visited.add(name);
  }

  for (const table of tables) visit(table.name);
  return sorted;
}

// ---------------------------------------------------------------------------
// PostgreSQL
// ---------------------------------------------------------------------------

function normalizePgType(dataType, udtName) {
  if (udtName === '_text' || udtName === '_varchar') return 'text[]';
  if (dataType === 'ARRAY') return 'text[]';
  if (dataType === 'json' || dataType === 'jsonb') return 'json';
  if (dataType === 'boolean') return 'boolean';
  if (dataType.startsWith('timestamp')) return 'timestamp';
  if (dataType === 'integer' || dataType === 'bigint' || dataType === 'smallint') return 'integer';
  if (dataType === 'real' || dataType === 'double precision' || dataType === 'numeric') return 'real';
  return 'text';
}

function createPgConnection(url) {
  const sql = postgres(url);

  return {
    type: 'postgresql',

    async introspect() {
      const tables = await sql`
        SELECT table_name FROM information_schema.tables
        WHERE table_schema = 'public' AND table_type = 'BASE TABLE'
      `;

      const result = [];

      for (const { table_name } of tables) {
        if (SKIP_TABLES.has(table_name)) continue;

        const cols = await sql`
          SELECT column_name, data_type, udt_name
          FROM information_schema.columns
          WHERE table_schema = 'public' AND table_name = ${table_name}
          ORDER BY ordinal_position
        `;
        const columns = cols.map(c => ({
          name: c.column_name,
          type: normalizePgType(c.data_type, c.udt_name),
        }));

        const fks = await sql`
          SELECT DISTINCT ccu.table_name AS referenced_table
          FROM information_schema.table_constraints tc
          JOIN information_schema.constraint_column_usage ccu
            ON tc.constraint_name = ccu.constraint_name AND tc.table_schema = ccu.table_schema
          WHERE tc.table_schema = 'public'
            AND tc.table_name = ${table_name}
            AND tc.constraint_type = 'FOREIGN KEY'
            AND ccu.table_name != ${table_name}
        `;
        const dependsOn = fks.map(f => f.referenced_table);

        result.push({ name: table_name, columns, dependsOn });
      }

      return result;
    },

    async readAll(table) {
      return (await sql`SELECT * FROM ${sql(table)} ORDER BY id`);
    },

    async insertRows(table, rows, columns) {
      if (rows.length === 0) return 0;

      const colMap = new Map(columns.map(c => [c.name, c.type]));
      let inserted = 0;

      for (const row of rows) {
        const colNames = Object.keys(row);
        const values = colNames.map(col => {
          const val = row[col];
          if (val === null || val === undefined) return null;
          const colType = colMap.get(col);

          // SQLite unix timestamps → PG Date
          if (colType === 'timestamp' && typeof val === 'number') {
            return new Date(val * 1000);
          }
          // SQLite 0/1 → PG boolean
          if (colType === 'boolean' && typeof val === 'number') {
            return val === 1;
          }
          // SQLite JSON text → PG json object
          if (colType === 'json' && typeof val === 'string') {
            try { return JSON.parse(val); } catch { return val; }
          }
          // SQLite JSON text → PG text[] array
          if (colType === 'text[]' && typeof val === 'string') {
            try { return JSON.parse(val); } catch { return val; }
          }
          return val;
        });

        await sql`INSERT INTO ${sql(table)} (${sql(colNames)}) VALUES (${sql(values)})`;
        inserted++;
      }

      // Reset serial sequences to avoid ID conflicts
      try {
        await sql`SELECT setval(pg_get_serial_sequence(${table}, 'id'), COALESCE((SELECT MAX(id) FROM ${sql(table)}), 0))`;
      } catch {
        // Table might not have a serial id
      }

      return inserted;
    },

    async close() {
      await sql.end();
    },
  };
}

// ---------------------------------------------------------------------------
// SQLite
// ---------------------------------------------------------------------------

function createSqliteConnection(path, options = {}) {
  const db = new Database(path);

  if (options?.runMigrations) {
    console.log('  Ensuring SQLite target schema with Drizzle migrations...');
    try {
      const drizzleDb = sqliteDrizzle(db);
      sqliteMigrate(drizzleDb, { migrationsFolder: SQLITE_MIGRATIONS_PATH });
      console.log('  SQLite schema is up to date.');
    } catch (error) {
      console.error('  Failed to run SQLite migrations:', error);
      db.close();
      throw error;
    }
  }

  db.pragma('journal_mode = WAL');
  db.pragma('foreign_keys = OFF'); // Disable during bulk insert

  return {
    type: 'sqlite',

    async introspect() {
      const tables = db.prepare(
        `SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name`
      ).all();

      const result = [];

      for (const { name } of tables) {
        if (SKIP_TABLES.has(name)) continue;

        const cols = db.prepare(`PRAGMA table_info('${name}')`).all();
        const columns = cols.map(c => ({
          name: c.name,
          // SQLite declared types don't distinguish timestamp/boolean/json from integer/text,
          // so we keep it simple — conversions are driven by the PG side's type info
          type: c.type?.toLowerCase() || 'text',
        }));

        const fks = db.prepare(`PRAGMA foreign_key_list('${name}')`).all();
        const dependsOn = [...new Set(fks.map(f => f.table).filter(t => t !== name))];

        result.push({ name, columns, dependsOn });
      }

      return result;
    },

    async readAll(table) {
      return db.prepare(`SELECT * FROM "${table}" ORDER BY id`).all();
    },

    async insertRows(table, rows, sourceColumns) {
      if (rows.length === 0) return 0;

      // When inserting into SQLite, convert PG native types to SQLite equivalents.
      // Most PG values carry their JS types (Date, boolean, Array, object) and can
      // be converted without metadata. The exception is PG json columns: the driver
      // auto-parses them, so a JSON string "hello" becomes the bare JS string hello,
      // indistinguishable from a text column. Drizzle's json-mode text columns
      // expect valid JSON text, so we need source column info to re-stringify them.
      const jsonCols = new Set(
        (sourceColumns || []).filter(c => c.type === 'json').map(c => c.name)
      );

      const colNames = Object.keys(rows[0]);
      const placeholders = colNames.map(() => '?').join(', ');
      const stmt = db.prepare(`INSERT INTO "${table}" (${colNames.join(', ')}) VALUES (${placeholders})`);

      const insertMany = db.transaction((rowList) => {
        let count = 0;
        for (const row of rowList) {
          const values = colNames.map(col => {
            const val = row[col];
            if (val === null || val === undefined) return null;
            // PG json → re-stringify so Drizzle's json-mode text columns can parse
            if (jsonCols.has(col)) {
              return JSON.stringify(val);
            }
            // PG Date → Unix integer
            if (val instanceof Date) {
              return Math.floor(val.getTime() / 1000);
            }
            // PG boolean → 0/1
            if (typeof val === 'boolean') {
              return val ? 1 : 0;
            }
            // PG array or json object → JSON string
            if (Array.isArray(val) || (typeof val === 'object' && val !== null)) {
              return JSON.stringify(val);
            }
            return val;
          });
          stmt.run(...values);
          count++;
        }
        return count;
      });

      return insertMany(rows);
    },

    async close() {
      db.close();
    },
  };
}

// ---------------------------------------------------------------------------
// Migration
// ---------------------------------------------------------------------------

async function migrate(fromStr, toStr) {
  const from = parseConnectionString(fromStr);
  const to = parseConnectionString(toStr);

  if (from.type === to.type) {
    console.error('Source and target must be different database types (one PostgreSQL, one SQLite).');
    process.exit(1);
  }

  console.log(`\nMigrating: ${from.type} → ${to.type}`);
  console.log(`  From: ${from.type === 'postgresql' ? from.url.replace(/\/\/[^:]*:[^@]*@/, '//****:****@') : from.url}`);
  console.log(`  To:   ${to.type === 'postgresql' ? to.url.replace(/\/\/[^:]*:[^@]*@/, '//****:****@') : to.url}\n`);

  const source = from.type === 'postgresql'
    ? createPgConnection(from.url)
    : createSqliteConnection(from.url);

  const target = to.type === 'postgresql'
    ? createPgConnection(to.url)
    : createSqliteConnection(to.url, { runMigrations: true });

  try {
    // Introspect both databases to discover tables, columns, and types
    const sourceTables = await source.introspect();
    const targetTables = await target.introspect();
    const targetTableNames = new Set(targetTables.map(t => t.name));
    const sourceColMap = new Map(sourceTables.map(t => [t.name, t.columns]));
    const targetColMap = new Map(targetTables.map(t => [t.name, t.columns]));

    // Only migrate tables that exist in both source and target
    const migratable = sourceTables.filter(t => targetTableNames.has(t.name));
    const sorted = topologicalSort(migratable);

    console.log(`  Found ${sorted.length} tables to migrate\n`);

    let totalRows = 0;
    let tablesWithData = 0;

    for (const table of sorted) {
      try {
        const rows = await source.readAll(table.name);
        if (rows.length === 0) {
          console.log(`  - ${table.name}: 0 rows (empty)`);
          continue;
        }

        // PG→SQLite: pass source columns so SQLite can re-stringify json values
        // SQLite→PG: pass target columns so PG can convert types back
        const sourceCols = sourceColMap.get(table.name) || table.columns;
        const targetCols = targetColMap.get(table.name) || table.columns;
        const inserted = await target.insertRows(table.name, rows, target.type === 'sqlite' ? sourceCols : targetCols);
        totalRows += inserted;
        tablesWithData++;
        console.log(`  ✓ ${table.name}: ${inserted} rows`);
      } catch (error) {
        const msg = error instanceof Error ? error.message : String(error);
        if (msg.includes('no such table') || msg.includes('does not exist') || msg.includes('relation')) {
          console.log(`  - ${table.name}: skipped (not found)`);
        } else {
          console.error(`  ✗ ${table.name}: ${msg}`);
          throw error;
        }
      }
    }

    console.log(`\nMigration complete: ${totalRows} total rows across ${tablesWithData} tables.`);
  } finally {
    await source.close();
    await target.close();
  }
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

const args = process.argv.slice(2);
let fromUrl = '';
let toUrl = '';

for (let i = 0; i < args.length; i++) {
  if (args[i] === '--from' && args[i + 1]) fromUrl = args[++i];
  else if (args[i] === '--to' && args[i + 1]) toUrl = args[++i];
}

if (!fromUrl || !toUrl) {
  console.log(`
Sidereal Database Migration Tool

Migrates all data between PostgreSQL and SQLite in either direction.
Tables, columns, and types are discovered automatically from database
metadata — no updates needed when the schema changes.

Usage:
  node tools/scripts/migrate-db.js --from <source> --to <target>

Examples:
  # PostgreSQL → SQLite
  node tools/scripts/migrate-db.js \\
    --from postgresql://sidereal:password@localhost:5432/sidereal \\
    --to sqlite:sidereal.db

  # SQLite → PostgreSQL
  node tools/scripts/migrate-db.js \\
    --from sqlite:local.db \\
    --to postgresql://sidereal:password@localhost:5432/sidereal

Note: SQLite targets run the bundled Drizzle migrations automatically to ensure their schema exists.
      PostgreSQL targets must still be created ahead of time.
`);
  process.exit(1);
}

migrate(fromUrl, toUrl).catch((error) => {
  console.error('\nMigration failed:', error.message);
  process.exit(1);
});
