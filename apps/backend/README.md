# Sidereal Backend

Hono-based API backend with PostgreSQL database and Drizzle ORM.

## Technology Stack

- **Hono**: Fast, lightweight web framework
- **Node.js 24**: Runtime with native TypeScript support
- **PostgreSQL**: Relational database
- **Drizzle ORM**: TypeScript ORM
- **Vitest**: Testing framework

## Development

### Install Dependencies

From the project root:
```bash
pnpm install
```

### Run Development Server

```bash
pnpm dev
```

The API will be available at http://localhost:3001 with hot-reload enabled using Node's native `--watch` flag.

### Available Scripts

- `pnpm dev` - Start development server with watch mode
- `pnpm build` - Build for production
- `pnpm test` - Run tests
- `pnpm test:coverage` - Run tests with coverage
- `pnpm lint` - Lint code
- `pnpm type-check` - Type check without building
- `pnpm db:generate` - Generate migration files from schema
- `pnpm db:migrate` - Run pending migrations
- `pnpm db:push` - Push schema changes directly to database
- `pnpm db:studio` - Open Drizzle Studio for database management

## Project Structure

```
apps/backend/
├── src/
│   ├── index.ts          # Application entry point
│   ├── index.test.ts     # Example test
│   └── db/
│       ├── schema.ts     # Database schema definitions
│       ├── connection.ts # Database connection utilities
│       └── migrate.ts    # Migration runner
├── drizzle/              # Generated migration files
├── drizzle.config.ts     # Drizzle ORM configuration
├── vitest.config.ts      # Vitest configuration
└── Dockerfile            # Docker container configuration
```

## API Endpoints

### Health Check
```
GET /health
```

Returns API and database health status:
```json
{
  "status": "healthy",
  "timestamp": "2025-11-17T00:00:00.000Z",
  "service": "backend"
}
```

## Database Migrations

### Workflow

1. **Update schema** in `src/db/schema.ts`

2. **Generate migration**
   ```bash
   pnpm db:generate
   ```

3. **Review migration** in `drizzle/` directory

4. **Run migration**
   ```bash
   pnpm db:migrate
   ```

5. **Verify in Adminer** at http://localhost:8080

### Example: Adding a Table

```typescript
// src/db/schema.ts
import { pgTable, serial, text, timestamp } from 'drizzle-orm/pg-core';

export const users = pgTable('users', {
  id: serial('id').primaryKey(),
  email: text('email').notNull().unique(),
  createdAt: timestamp('created_at').defaultNow().notNull(),
});
```

## Environment Variables

Create a `.env` file based on `.env.example`:

```bash
DATABASE_URL=postgresql://postgres:postgres@db:5432/sidereal_dev
PORT=3001
NODE_ENV=development
FRONTEND_URL=http://localhost:3000
```

## Docker Development

The backend runs in a Docker container with volume mounts for hot-reload:

```bash
# From project root
docker compose up backend
```

## Testing

```bash
# Run tests in watch mode
pnpm test

# Run tests with coverage
pnpm test:coverage
```

## Adding Endpoints

```typescript
// src/index.ts
import { Hono } from 'hono';

const app = new Hono();

app.get('/api/items', async (c) => {
  const items = await db.query.items.findMany();
  return c.json(items);
});

app.post('/api/items', async (c) => {
  const body = await c.req.json();
  const newItem = await db.insert(items).values(body).returning();
  return c.json(newItem, 201);
});
```

## Database Access

### Using Drizzle Studio

```bash
pnpm db:studio
```

Opens a web UI at http://localhost:4983 for browsing and editing database records.

### Using Adminer

Navigate to http://localhost:8080 and connect with:
- System: PostgreSQL
- Server: db
- Username: postgres
- Password: postgres
- Database: sidereal_dev
