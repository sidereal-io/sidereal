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

## Start the instance

```sh
docker compose up -d --build --wait
```

The first run builds the app image, which takes several minutes. The command returns when the app and the database are both healthy.

Open `http://localhost:5000/`. The page shows `healthy`.

Compose stops with an error that names `POSTGRES_PASSWORD` when `.env` is missing or the password is empty.

## Stop, restart, and update

| You want to | Run | What happens to your data |
|---|---|---|
| Stop the instance | `docker compose stop` | Kept |
| Start it again | `docker compose up -d --wait` | Kept |
| Remove the containers | `docker compose down` | Kept |
| Create the containers again | `docker compose down`, then `docker compose up -d --wait` | Kept |
| Update to newer code | `git pull`, then `docker compose up -d --build --wait` | Kept |

The database and the storage folder live in two named Docker volumes, `database` and `storage`. Removing the containers does not remove the volumes.

## Delete the data

Only this command deletes your data:

```sh
docker compose down --volumes
```

It removes the containers and both volumes. No other command in this guide deletes data.

## Things to know

### Changing the password later does not change the database's password

PostgreSQL reads `POSTGRES_PASSWORD` only on the first start, when it creates the database. If you change the value in `.env` afterwards, the app uses the new password, the database still expects the old one, and the app fails to start. Put the old value back to fix it.

### Compose can print your password

`docker compose config` prints the settings it resolved, including the password. The app's own logs never print it.

### Run one instance per machine

Compose names an instance after the folder that holds `compose.yml`. Two clones in folders with the same name share their containers, volumes, and image. To run a second instance, set a different `COMPOSE_PROJECT_NAME` and a different `SIDEREAL_PORT` for it.

### Running the server outside the image

The image sets `WEB_ROOT` to the folder that holds the built web pages. The app user cannot write to that folder.

If you run the server without the image and set `WEB_ROOT` yourself, make sure only people you trust can write to that folder. The server serves whatever the folder holds, and it follows symbolic links.
