# Migration Guide: Skymmich → Sidereal

This guide covers what you need to do when upgrading from the Skymmich-branded release to the Sidereal rename.

This is a **cosmetic rebrand only** — no data is lost, no schema changes occurred, and application behavior is unchanged. However, several names that Docker and your environment depend on have changed, so a few manual steps are required.

---

## Quick summary of breaking changes

| What changed | Old name | New name |
|---|---|---|
| Environment variable | `SKYMMICH_PORT` | `SIDEREAL_PORT` |
| Docker volumes | `skymmich-config`, `skymmich-logs`, etc. | `sidereal-config`, `sidereal-logs`, etc. |
| SQLite default path (inside container) | `/app/config/skymmich.db` | `/app/config/sidereal.db` |
| PostgreSQL database name | `skymmich` | `sidereal` |
| PostgreSQL username | `skymmich` | `sidereal` |
| Container system user | `skymmich` | `sidereal` |
| Unraid appdata path | `/mnt/user/appdata/skymmich/` | `/mnt/user/appdata/sidereal/` |

---

## Docker (SQLite — default)

This is the most common setup. Your data lives in the `skymmich-config` Docker volume.

### 1. Stop the running container

```sh
docker compose down
```

### 2. Rename the Docker volumes

Docker volumes cannot be renamed directly. Copy the data into new volumes instead:

```sh
# Config (contains your database)
docker volume create sidereal-config
docker run --rm \
  -v skymmich-config:/from \
  -v sidereal-config:/to \
  alpine sh -c "cp -a /from/. /to/"

# Logs
docker volume create sidereal-logs
docker run --rm \
  -v skymmich-logs:/from \
  -v sidereal-logs:/to \
  alpine sh -c "cp -a /from/. /to/"

# Sidecars
docker volume create sidereal-sidecars
docker run --rm \
  -v skymmich-sidecars:/from \
  -v sidereal-sidecars:/to \
  alpine sh -c "cp -a /from/. /to/"

# Cache (safe to skip — it will be rebuilt)
docker volume create sidereal-cache
```

### 3. Rename the database file inside the volume

The SQLite database file was stored as `skymmich.db` and is now expected at `sidereal.db`:

```sh
docker run --rm -v sidereal-config:/data alpine \
  mv /data/skymmich.db /data/sidereal.db
```

If you set a custom `SQLITE_DB_PATH`, you can skip this step — the path you configured still works.

### 4. Update your `.env` file

If you had `SKYMMICH_PORT` set, rename it:

```sh
# Before
SKYMMICH_PORT=5000

# After
SIDEREAL_PORT=5000
```

### 5. Pull the new image and start

```sh
docker compose pull
docker compose up -d
```

### 6. Verify

```sh
docker compose logs -f sidereal
```

You should see the app start cleanly and report the correct database path.

### 7. Clean up old volumes (optional)

Once you have confirmed everything works, remove the old volumes:

```sh
docker volume rm skymmich-config skymmich-logs skymmich-sidecars skymmich-cache
```

---

## Docker (PostgreSQL)

If you were using the PostgreSQL override (`docker-compose.postgres.yml`), the database name and user have changed.

### Option A: Rename the database and user in place (least downtime)

Connect to your PostgreSQL container and run:

```sql
-- Rename the database
ALTER DATABASE skymmich RENAME TO sidereal;

-- Rename the user
ALTER USER skymmich RENAME TO sidereal;
```

Then update your `DATABASE_URL` environment variable:

```sh
# Before
DATABASE_URL=postgresql://skymmich:password@skymmich-db:5432/skymmich

# After
DATABASE_URL=postgresql://sidereal:password@sidereal-db:5432/sidereal
```

The PostgreSQL container itself is now named `sidereal-db` in the new compose file, so also rename that Docker volume:

```sh
docker volume create sidereal-database
docker run --rm \
  -v skymmich-database:/from \
  -v sidereal-database:/to \
  alpine sh -c "cp -a /from/. /to/"
```

### Option B: Dump and restore

```sh
# Dump from the old container
docker exec skymmich-db pg_dump -U skymmich skymmich > skymmich-backup.sql

# Start the new stack (creates empty sidereal DB)
docker compose -f docker-compose.yml -f docker-compose.postgres.yml up -d sidereal-db

# Restore into the new database
docker exec -i sidereal-db psql -U sidereal sidereal < skymmich-backup.sql
```

---

## Unraid

1. Stop the Skymmich container in the Unraid UI.
2. Move your appdata folder:
   ```sh
   mv /mnt/user/appdata/skymmich /mnt/user/appdata/sidereal
   ```
3. Rename the database file inside it:
   ```sh
   mv /mnt/user/appdata/sidereal/skymmich.db /mnt/user/appdata/sidereal/sidereal.db
   ```
4. Install the updated Sidereal template from Community Applications (or update the template XML path manually).
5. Update the container path in the template from `/mnt/user/appdata/skymmich/` to `/mnt/user/appdata/sidereal/`.
6. Start the container and verify the logs.

---

## Environment variables reference

| Old variable | New variable | Notes |
|---|---|---|
| `SKYMMICH_PORT` | `SIDEREAL_PORT` | External port mapping, defaults to `5000` |

All other environment variables (`DATABASE_URL`, `SQLITE_DB_PATH`, `IMMICH_URL`, etc.) are unchanged.

---

## Keeping custom `SQLITE_DB_PATH`

If you already set `SQLITE_DB_PATH` to a custom path (e.g. `/app/config/mydata.db`), that path still works and you do not need to rename the file. The new default only affects fresh installs.

---

## Rolling back

If you need to revert to the old image before completing migration:

1. Stop and remove the new container.
2. The old volumes (`skymmich-*`) are untouched — you did not delete them yet.
3. Start the old image with the old compose file.
