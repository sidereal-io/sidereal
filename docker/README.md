# Sidereal Docker Deployment

This directory contains all the necessary files for deploying Sidereal using Docker containers.

## Quick Start

### 1. Clone and Build

```bash
git clone <your-repo-url>
cd sidereal
docker compose up -d
```

No external database or `.env` file required — Sidereal uses a built-in SQLite database by default.

### 2. Environment Configuration (Optional)

Copy `docker/.env.docker.example` to `.env` in the project root to customize:

```bash
# Optional: Application configuration
SIDEREAL_PORT=5000
IMMICH_URL=http://your-immich-server:2283
IMMICH_API_KEY=your_immich_api_key_here
ASTROMETRY_API_KEY=your_astrometry_api_key_here
ENABLE_PLATE_SOLVING=true
PLATE_SOLVE_MAX_CONCURRENT=3
```

**Note**: API keys can also be configured via the admin web interface after startup.

### Using PostgreSQL Instead

To use PostgreSQL instead of the built-in SQLite database, layer the postgres override:

```bash
echo "POSTGRES_PASSWORD=your_secure_password" > .env
docker compose -f docker-compose.yml -f docker-compose.postgres.yml up -d
```

## Files Overview

### Core Files
- `Dockerfile` - Multi-stage build for Sidereal application
- `docker-compose.yml` - Default setup (SQLite, single container)
- `docker-compose.postgres.yml` - PostgreSQL override (layer on top)
- `startup.sh` - Container entry point script
- `.env.docker.example` - Docker environment variable template

### UnRAID Templates
- `unraid-templates/sidereal.xml` - Main application template

## UnRAID Installation

### Step 1: Install Sidereal
1. Go to Docker tab in UnRAID
2. Click "Add Container"
3. Use template URL: `https://raw.githubusercontent.com/mstelz/Sidereal/main/docker/unraid-templates/sidereal.xml`
4. Optionally add your Immich and Astrometry.net credentials
5. Apply and start container

### Step 2: Access Application
- Open web interface: `http://your-unraid-ip:2284`
- Configure admin settings
- Start syncing your astrophotography collection!

> To use PostgreSQL instead of the default SQLite database, set the `DATABASE_URL` field in the template to your PostgreSQL connection string.

## Docker Compose Commands

```bash
# Build and start services
docker compose up -d

# View logs
docker compose logs -f

# Stop services
docker compose down

# Rebuild and restart
docker compose up -d --build
```

## Container Architecture

```
┌─────────────────────────────────────┐
│        Sidereal Container           │
├─────────────────────────────────────┤
│  Frontend (React SPA)              │
│  Backend (Hono API)                │
│  Worker (Plate Solving)            │    ┌─────────────────────┐
│  Real-time Updates (WebSocket)     │    │  PostgreSQL         │
│  SQLite Database (built-in)        │◄──►│  (optional)         │
└─────────────────────────────────────┘    └─────────────────────┘
```

## Health Monitoring

### Sidereal Health Check
- Endpoint: `http://localhost:5000/api/health`
- Checks database connectivity and worker status
- 30-second intervals with 40-second startup period

### PostgreSQL Health Check (if using PostgreSQL)
- Command: `pg_isready -U sidereal -d sidereal`
- 10-second intervals with 30-second startup period

## Data Persistence

### Volumes
- **Sidereal Config + Database**: `/app/config` (includes `sidereal.db` when using SQLite)
- **Logs**: `/app/logs`
- **Sidecars**: `/app/sidecars`
- **Cache**: `/app/cache`

### Backup Strategy

**SQLite (default):**
```bash
# Backup database (just copy the file)
docker cp sidereal:/app/config/sidereal.db ./sidereal-backup.db

# Backup configuration
tar -czf sidereal-config-backup.tar.gz /mnt/user/appdata/sidereal/config

# Restore database
docker cp ./sidereal-backup.db sidereal:/app/config/sidereal.db
docker restart sidereal
```

**PostgreSQL (if using postgres override):**
```bash
# Backup database
docker exec sidereal-db pg_dump -U sidereal sidereal > backup.sql

# Restore database
docker exec -i sidereal-db psql -U sidereal sidereal < backup.sql
```

## Migrating Between SQLite and PostgreSQL

A migration script is included for moving data between database engines in either direction.

**Important:** SQLite targets automatically run the bundled Drizzle migrations during the copy. PostgreSQL targets must still have their schema created ahead of time (start Sidereal once with the postgres override so the tables exist).

### PostgreSQL → SQLite (switching to the new default)
```bash
# Stop the application first
docker compose down

# Run migration inside the container
docker run --rm \
  -v sidereal-config:/app/config \
  --network sidereal-network \
  ghcr.io/mstelz/sidereal:latest \
  node /app/dist/tools/scripts/migrate-db.js \
    --from postgresql://sidereal:password@sidereal-db:5432/sidereal \
    --to sqlite:/app/config/sidereal.db

# Then start with the default compose (no PostgreSQL)
docker compose -f docker-compose.prod.yml up -d
```

### SQLite → PostgreSQL
```bash
# Stop the application first
docker compose down

# Run migration inside the container
docker run --rm \
  -v sidereal-config:/app/config \
  --network sidereal-network \
  ghcr.io/mstelz/sidereal:latest \
  node /app/dist/tools/scripts/migrate-db.js \
    --from sqlite:/app/config/sidereal.db \
    --to postgresql://sidereal:password@sidereal-db:5432/sidereal

# Then start with the postgres override
docker compose -f docker-compose.prod.yml -f docker-compose.postgres.yml up -d
```

### Local Development
```bash
node tools/scripts/migrate-db.js \
  --from sqlite:local.db \
  --to postgresql://sidereal:password@localhost:5432/sidereal
```

### Automatic Migration During Container Startup

Set the `AUTO_DB_MIGRATE_FROM` environment variable on the Sidereal container to have the startup script run the migration automatically before the application boots. The target defaults to whichever database the container is configured to use (PostgreSQL when `DATABASE_URL` is set, otherwise the built-in SQLite database at `/app/config/sidereal.db`).

Optional environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `AUTO_DB_MIGRATE_FROM` | _(required)_ | Source connection string (e.g. `postgresql://...` or `sqlite:/app/config/sidereal.db`). |
| `AUTO_DB_MIGRATE_TO` | auto-detected | Override the destination connection string. |
| `AUTO_DB_MIGRATE_ONCE` | `true` | When `true`, the migration runs only the first time and writes a marker file. |
| `AUTO_DB_MIGRATE_MARKER` | `/app/config/.auto-db-migrated` | Marker file path used when `AUTO_DB_MIGRATE_ONCE=true`. Remove this file to re-run the automatic migration. |
| `AUTO_DB_MIGRATE_RESET_SQLITE` | `true` | When migrating into SQLite, delete the target DB file before copying data. Set to `false` to keep the existing file. |

Examples:

```bash
# Migrate once from PostgreSQL back to the built-in SQLite database
AUTO_DB_MIGRATE_FROM=postgresql://sidereal:password@sidereal-db:5432/sidereal \
  docker compose -f docker-compose.prod.yml up -d

# Migrate from SQLite to PostgreSQL before switching to the postgres override
AUTO_DB_MIGRATE_FROM=sqlite:/app/config/sidereal.db \
  DATABASE_URL=postgresql://sidereal:password@sidereal-db:5432/sidereal \
  docker compose -f docker-compose.prod.yml -f docker-compose.postgres.yml up -d
```

After the automatic migration completes the container continues starting normally. Remove the environment variable (or delete the marker file) once the data has been copied.

## Troubleshooting

### Container Won't Start
1. Check logs: `docker compose logs sidereal`
2. Check environment variables
3. Ensure ports aren't in use
4. Verify volume permissions (check PUID/PGID settings)

### Worker Not Running
1. Check environment variable: `ENABLE_PLATE_SOLVING=true`
2. View worker status in health endpoint
3. Check Astrometry.net API key configuration

### Database Issues
1. **SQLite**: Check that `/app/config` volume is mounted and writable
2. **PostgreSQL**: Verify the PostgreSQL container is running and `DATABASE_URL` is correct

### Performance Issues
1. Monitor container resources
2. Adjust `PLATE_SOLVE_MAX_CONCURRENT` setting
3. Check available disk space

## Security Considerations

**Critical Security Notes:**

- **Secure API key storage** - Set via environment variables or admin interface
- **No secrets in images** - Container images contain no embedded secrets
- **Regular updates** - Keep containers updated with latest security patches
- **Monitor access** - Review logs for unauthorized access attempts
- **File permissions** - Containers run as non-root user (`sidereal`)
- If using PostgreSQL: use a strong password and don't expose the PostgreSQL port externally

**Environment Variables vs Admin Interface:**
- Environment variables take precedence for initial configuration
- Admin interface settings are stored in the database
- Both methods are secure when properly configured

## Development

For development with hot reload:

```bash
# Run application locally (uses SQLite by default)
npm run dev
```
