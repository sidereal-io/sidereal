import { drizzle as pgDrizzle } from 'drizzle-orm/node-postgres';
import pg from 'pg';
const { Pool } = pg;
import * as pgSchema from '@shared/db/pg-schema';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import fs from 'node:fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const SQLITE_MIGRATIONS_CANDIDATES = [
  path.resolve(process.cwd(), 'tools/migrations/sqlite'),
  path.resolve(process.cwd(), 'dist/tools/migrations/sqlite'),
  path.resolve(__dirname, '../../..', 'tools/migrations/sqlite'),
];
const SQLITE_MIGRATIONS_PATH = SQLITE_MIGRATIONS_CANDIDATES.find(candidate =>
  fs.existsSync(path.join(candidate, 'meta', '_journal.json'))
) || SQLITE_MIGRATIONS_CANDIDATES[0];

let db: any;
let schema: any;
let initialized = false;
let sqliteDbPath: string | null = null;
export const isPostgres = !!process.env.DATABASE_URL;

async function initializeDatabase() {
  if (initialized) return;

  // Check if DATABASE_URL is provided (PostgreSQL)
  if (process.env.DATABASE_URL) {
    console.log('Using PostgreSQL database');
    const pool = new Pool({
      connectionString: process.env.DATABASE_URL,
    });
    schema = pgSchema;
    db = pgDrizzle(pool, { schema });
  } else {
    // Use built-in SQLite database
    try {
      const dbPath = process.env.SQLITE_DB_PATH || (process.env.NODE_ENV === 'production' ? '/app/config/sidereal.db' : 'local.db');
      console.log(`Using SQLite database: ${dbPath}`);

      // Dynamic imports to handle missing dependencies gracefully
      const { drizzle: sqliteDrizzle } = await import('drizzle-orm/better-sqlite3');
      const { migrate } = await import('drizzle-orm/better-sqlite3/migrator');
      const Database = (await import('better-sqlite3')).default;
      const sqliteSchema = await import('@shared/db/sqlite-schema');

      sqliteDbPath = dbPath;
      const sqlite = new Database(dbPath);
      schema = sqliteSchema;
      db = sqliteDrizzle(sqlite, { schema });
      
      // Run migrations (Drizzle tracks which have been applied and only runs new ones)
      try {
        migrate(db, { migrationsFolder: SQLITE_MIGRATIONS_PATH });
        console.log('Migrations completed successfully');
      } catch (migrationError) {
        console.error('Migration error:', migrationError);
        throw migrationError;
      }
    } catch (error) {
      throw new Error(
        'No database configuration found. Please set DATABASE_URL environment variable for PostgreSQL connection.'
      );
    }
  }

  initialized = true;
}

// Initialize immediately
await initializeDatabase();

export { db, schema, sqliteDbPath };
