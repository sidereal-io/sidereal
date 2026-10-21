# Install Sidereal with Docker Compose

This guide starts a private Sidereal instance on one machine. Docker Compose runs two containers: the Sidereal app and its PostgreSQL database.

> [!WARNING]
> This installs the rebuild of Sidereal from the `main` branch. Today it shows only the server's health. To manage your images now, use the [`v0.x` branch](https://github.com/sidereal-io/sidereal/tree/v0.x).

## Before you start

You need:

- Docker Engine or Docker Desktop, with Compose 2.20 or newer. Run `docker compose version` to see your release.
- A clone of this repository. Docker builds the app image from it.
- `openssl`, to generate a password.

Run every command in this guide from the repository root.

## The app has no sign-in yet

Anyone who can reach the app's port can open it. The app listens on every network interface of your machine.

Until sign-in exists, the app serves only its web pages and two health checks. It holds none of your images and accepts no changes.

Do not publish the port to the internet.

## Create your settings file

1. Copy the example file:

   ```sh
   cp .env.example .env
   ```

2. Generate a database password:

   ```sh
   openssl rand -hex 32
   ```

3. Open `.env` and paste the password after `POSTGRES_PASSWORD=`.

The password must hold only letters and digits. Other characters can break the database address, and the app will not start.

Git ignores `.env`. Keep it private: it holds your password.

| Setting | Default | Purpose |
|---|---|---|
| `POSTGRES_PASSWORD` | None. You must set it | The database password |
| `SIDEREAL_PORT` | `5000` | The port on your machine that the app listens on |
| `COMPOSE_FILE` | Not set | The Compose files to use. Set it only to keep data in host directories |
| `SIDEREAL_STORAGE_PATH` | Not set | The host directory for managed storage, with `compose.storage-bind.yml` |
| `SIDEREAL_DATABASE_PATH` | Not set | The host directory for PostgreSQL data, with `compose.database-bind.yml` |

To keep data in Docker volumes, leave the last three settings unset. To choose host directories, see [Choose where your data lives](#choose-where-your-data-lives).

## Start the instance

```sh
docker compose up -d --build --wait
```

The first run builds the app image, which takes several minutes. The command returns when the app and the database are both healthy.

Open `http://localhost:5000/`. The page shows `healthy`.

Compose stops with an error that names `POSTGRES_PASSWORD` when `.env` is missing or the password is empty.

The first start also prepares storage. The app writes one small file, `.sidereal-storage.json`, into the empty storage location. It records the same identity in the database. This pairing lets later starts tell the right storage from an empty substitute. See [Startup checks your storage](#startup-checks-your-storage).

## Choose where your data lives

Sidereal keeps data in two locations:

- **Storage** holds managed files. Today it holds only the storage marker, `.sidereal-storage.json`.
- **The database** holds PostgreSQL data, including the storage pairing.

By default, both live in Docker volumes named `storage` and `database`. You need no settings for this.

You can keep either location, or both, in a directory on your machine instead. Choose each one independently.

### Prepare a host directory

Do this before the first start that uses the directory.

1. Mount the disk you want to use.
2. Create a new, empty, dedicated subdirectory on that disk, below its mount point. For example, create `/mnt/photos/sidereal-storage`, not `/mnt/photos`.
3. Create the subdirectory only while the disk is mounted. Never create it again while the disk is unmounted.
4. Give the app access, as the next section describes.

The dedicated subdirectory protects you when the disk fails to mount. The subdirectory then does not exist, so Compose refuses to start the service. Compose never creates a missing directory, and it never falls back to a Docker volume.

A disk's mount point itself is a poor choice. It still exists while the disk is unmounted. A newly formatted disk can also hold `lost+found`, and startup refuses storage that holds other files.

### Give the app access

The app runs as user and group `10001:10001`. It needs to create, write, sync, and delete files in the storage directory. Grant that access to the directory itself, for example:

```sh
sudo chown 10001:10001 /mnt/photos/sidereal-storage
```

Or keep your own ownership and add an access control entry:

```sh
setfacl -m u:10001:rwx /mnt/photos/sidereal-storage
```

Change only the new directory. Never change ownership or permissions of a whole disk or tree, and never run the app with more privileges. The app never changes ownership or permissions itself.

The database directory needs no preparation beyond being new and empty. The PostgreSQL image sets up its own files and user.

Some hosts need more:

- **Docker Desktop** (macOS and Windows): share the directory's drive or folder with Docker in Docker Desktop's file sharing settings.
- **Rootless Docker or Podman**: user IDs in the container map to a range of IDs on your machine. Grant access to the host ID that container user `10001` maps to. Usually that is the first ID of your range in `/etc/subuid`, plus 10000.
- **SELinux**: label the new directory so containers may use it, for example `chcon -t container_file_t /mnt/photos/sidereal-storage`. Do not turn SELinux off.
- **Access control lists**: an entry that denies `10001` overrides the owner bits. Check with `getfacl`.

Storage must support creating files exclusively and syncing files and folders to disk. Some network shares do not. On such storage, the first start fails with an error that names a sync step.

Keep PostgreSQL data on a local disk that suits a database. Do not put it on a network share.

### Select the directories in `.env`

Add the settings to `.env`, so every later command uses them. This example keeps both locations in host directories:

```sh
COMPOSE_FILE=compose.yml:compose.storage-bind.yml:compose.database-bind.yml
SIDEREAL_STORAGE_PATH=/mnt/photos/sidereal-storage
SIDEREAL_DATABASE_PATH=/srv/sidereal-database
```

To move only one location, list only its file after `compose.yml`:

| Keep in a host directory | `COMPOSE_FILE` | Path setting |
|---|---|---|
| Storage only | `compose.yml:compose.storage-bind.yml` | `SIDEREAL_STORAGE_PATH` |
| Database only | `compose.yml:compose.database-bind.yml` | `SIDEREAL_DATABASE_PATH` |
| Both | `compose.yml:compose.storage-bind.yml:compose.database-bind.yml` | Both |

On Windows, separate the files with `;` instead of `:`, for example `compose.yml;compose.storage-bind.yml`.

Use absolute paths. Compose reads a relative path from the repository root.

Then start the instance with the same command as before:

```sh
docker compose up -d --build --wait
```

You can name the files on the command line instead of setting `COMPOSE_FILE`. Then name them in every command, in the same order:

```sh
docker compose -f compose.yml -f compose.storage-bind.yml -f compose.database-bind.yml up -d --build --wait
```

To check your selection, run `docker compose config` and read the `volumes` entries of `app` and `db`. That command also prints your password, so do not share its output.

Compose stops with an error that names the setting when a selected path is empty or unset. It stops with `bind source path does not exist` when the directory is missing.

### Host paths and container paths

The app sees storage at `/var/lib/sidereal/storage`, whichever location you chose. Its error messages name that container path. It maps to `SIDEREAL_STORAGE_PATH` when you use the storage override, and to the `storage` volume otherwise.

### Stop before you change a mount

Stop the instance with `docker compose stop` before you unmount, remount, or move a disk. Start it again once the disk is back. The app checks storage only when it starts.

## Startup checks your storage

Each start compares the storage marker with the pairing in the database before it serves any page:

| Database | Storage | What startup does |
|---|---|---|
| Not paired yet | Empty | Writes a new marker and records its identity. This is a first start |
| Not paired yet | Only a valid marker | Records that marker's identity. This finishes an interrupted first start |
| Not paired yet | Other files | Refuses. It does not adopt or change the files |
| Paired | The matching marker | Starts normally |
| Paired | No marker | Refuses with `storage marker is missing` |
| Paired | Another or a damaged marker | Refuses and changes neither identity |

Startup never replaces a marker or a pairing. It never deletes files it did not create.

### When startup reports a missing marker

The database remembers storage that is not there. Usually a disk is not mounted, or a path setting names the wrong directory.

1. Stop the instance: `docker compose stop`.
2. Mount the disk, or correct the path setting in `.env`.
3. Start the instance again.

Do not create a new marker, and do not empty the storage location to make startup pass. Either would pair the database with storage that does not hold its files.

### What the marker does not do

- It identifies storage. It does not prove that files are intact.
- A database that is not paired yet accepts any directory that holds only a valid marker.
- It does not stop two instances from using the same storage. Give each instance its own storage and database.
- It does not detect a mount that changes while the app runs.

## Stop, restart, and update

| You want to | Run | What happens to your data |
|---|---|---|
| Stop the instance | `docker compose stop` | Kept |
| Start it again | `docker compose up -d --wait` | Kept |
| Remove the containers | `docker compose down` | Kept |
| Create the containers again | `docker compose down`, then `docker compose up -d --wait` | Kept |
| Update to newer code | `git pull`, then `docker compose up -d --build --wait` | Kept |

Removing the containers keeps both data locations, whether they are Docker volumes or host directories.

## What Sidereal keeps

| Item | Where it lives | After `docker compose down --volumes` |
|---|---|---|
| `.env`, with your password and mount settings | The repository root | Kept |
| Storage, with the storage marker | The `storage` volume, or `SIDEREAL_STORAGE_PATH` | Deleted from the volume. Kept in a host directory |
| The database, with the storage pairing | The `database` volume, or `SIDEREAL_DATABASE_PATH` | Deleted from the volume. Kept in a host directory |

Back up `.env` and both data locations together. The marker and the pairing must match when you restore them.

## Delete the data

Only this command deletes data:

```sh
docker compose down --volumes
```

It removes the containers and the Docker volumes. It never deletes a host directory. No other command in this guide deletes data.

When you use only one host directory, this command deletes one location and keeps the other:

- **Database kept, storage deleted**: every later start refuses with `storage marker is missing`.
- **Database deleted, storage kept with only its marker**: the new database adopts that marker on the next start.
- **Database deleted, storage kept with other files**: every later start refuses.

To delete everything, run `docker compose down --volumes`, then delete the contents of each host directory you use.

### When storage is lost for good

If paired storage cannot come back, choose one:

- Restore both locations from copies taken at the same time.
- Start fresh: run `docker compose down --volumes`, then empty or replace both data locations.

Startup cannot tell a first start from losing both locations at once. When the database and storage are both empty, it starts fresh. Keep both locations safe together.

## Things to know

### Changing the password later does not change the database's password

PostgreSQL reads `POSTGRES_PASSWORD` only on the first start, when it creates the database. If you change the value in `.env` afterwards, the app uses the new password, the database still expects the old one, and the app fails to start. Put the old value back to fix it.

### Compose can print your password

`docker compose config` prints the settings it resolved, including the password. The app's own logs never print it.

### Going back to an older release

After an update, the database holds a newer migration history. An older app refuses to start against it. To go back, restore copies of both data locations taken before the update. Do not edit the migration history, and do not delete data to force an older release to start.

### Run one instance per machine

Compose names an instance after the folder that holds `compose.yml`. Two clones in folders with the same name share their containers, volumes, and image. To run a second instance, set a different `COMPOSE_PROJECT_NAME` and a different `SIDEREAL_PORT` for it.

### Running the server outside the image

The image sets `WEB_ROOT` to the folder that holds the built web pages. The app user cannot write to that folder.

If you run the server without the image and set `WEB_ROOT` yourself, make sure only people you trust can write to that folder. The server serves whatever the folder holds, and it follows symbolic links.

## Troubleshoot a failed start or an unhealthy app

Run `docker compose ps` to see the containers' health. Read `docker compose logs app` for the cause.
Add `-t` to the logs command when you need timestamps.

A running but `unhealthy` app has failed its recent readiness checks. `/healthz` still answers while HTTP runs; `/readyz` checks the database and schema.
Docker checks readiness every 10 seconds and marks the app unhealthy after three failed checks. It can take up to 40 seconds to notice an outage.
Docker does not restart a container just because it is unhealthy. The app recovers on its own after the database becomes usable.
The logs report each change once, for example `readiness: not ready (database unreachable)`, then `readiness: ready`.
Repeated checks with the same result add no line. A stalled schema check reports `readiness: not ready (timed out)`.

A failed startup exits. Docker's restart policy starts the app again, so a database that starts late needs no action.
After correcting `.env`, run `docker compose up -d --wait` to apply the settings and retry.

| Message starts with | Likely cause | Recovery step |
|---|---|---|
| `missing database configuration` | `DATABASE_URL` is absent | Set the dedicated PostgreSQL database URL |
| `invalid database configuration` | Invalid URL, TLS mode, or timeout | Correct the named setting; timeout seconds must be positive |
| `database TLS certificate file unreadable` | A certificate path is missing, unreadable, not a regular file, or larger than 1 MiB | Check `sslrootcert`, `sslcert`, and `sslkey` paths and read access inside the container |
| `database unreachable` | DNS, a refused connection, a silent host, or PostgreSQL starting or shutting down | Start PostgreSQL or wait for it to become available |
| `database login rejected` | Wrong credentials or a PostgreSQL client access rule | Correct the configured credentials or access rule; changing `.env` does not change an existing database's password |
| `database TLS verification failed` | An untrusted certificate or the wrong hostname | Configure a trusted CA and the hostname on the database certificate |
| `database does not exist` | The configured database name is absent | Create the dedicated database named in the configuration |
| `database connection failed` | Another connection fault | Check the PostgreSQL service's logs |
| `database permission denied` | The configured role lacks required privileges | Grant that role the required database, schema, and table privileges |
| `unsupported database version` | PostgreSQL is outside the supported major version | Use PostgreSQL 18; never downgrade existing data |
| `database is not an empty or owned Sidereal database` | The URL names unrelated data or an unrecognized identity | Correct the URL to use this instance's dedicated database |
| `incompatible migration history` | A newer release or damaged migration records | Use that newer release or a later one; investigate damaged or unexplained records while keeping all data as it is |
| `incompatible database schema` | The schema differs from the expected Sidereal schema | Investigate the difference before restoring a known compatible copy of both data locations |
| `database startup deadline exceeded` | A startup lock or later startup query stalled | Check competing starts and database locks before increasing `DATABASE_STARTUP_TIMEOUT_SECONDS` |
| `database migration failed` | Migration permissions, a lock, or the migration deadline | Correct the reported cause in PostgreSQL's logs and restart; adjust `DATABASE_MIGRATION_TIMEOUT_SECONDS` if needed |
| `storage` or `invalid storage configuration` | A missing mount, damaged or mismatched marker, or denied storage operation | Stop the instance and restore the matching mount and app-user access; follow [Startup checks your storage](#startup-checks-your-storage) |

Deleting volumes or data directories is never a recovery step: it destroys the library.
Do not run `docker compose down --volumes` to fix a failed start.
Editing migration records or the schema by hand to force a start is never a recovery step.
A migration history from a newer release needs that release or a later one, never an older binary.
