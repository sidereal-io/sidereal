# syntax=docker/dockerfile:1
# The build context is the repo root. .dockerignore sends server/ and web/ only.

# ---- web stage: build the web shell ----
FROM node:26-bookworm-slim AS web
WORKDIR /build/web
# pnpm 12 switches itself to the release that web/package.json pins, and the
# frozen install checks that release against the lockfile.
RUN npm install --global pnpm@12
COPY web/package.json web/pnpm-lock.yaml ./
RUN pnpm --version && pnpm install --frozen-lockfile
COPY web/ ./
RUN pnpm build

# ---- server stage: compile the server binary in release mode ----
FROM rust:1-slim-bookworm AS server
WORKDIR /build
# `rustup toolchain install` installs the toolchain that rust-toolchain.toml
# names (the stable channel). It shares a step with the build, so a cached
# layer never keeps an older compiler apart from the build it made.
COPY server/ ./
RUN rustup toolchain install && cargo build --release -p sidereal-server

# ---- runtime stage: slim image with the binary and the web shell ----
FROM debian:bookworm-slim AS runtime
# curl backs the HEALTHCHECK; ca-certificates for future outbound TLS.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
# Unprivileged user. It owns the storage folder, so a new named volume
# mounted there starts out writable.
RUN useradd --uid 10001 --user-group --no-create-home app \
    && install -d -o app -g app /var/lib/sidereal/storage
COPY --from=server /build/target/release/sidereal-server /usr/local/bin/sidereal-server
# Root owns the web shell, so the app user cannot change what the server serves.
COPY --from=web /build/web/dist /usr/share/sidereal/web
USER app
ENV PORT=5000 \
    WEB_ROOT=/usr/share/sidereal/web
EXPOSE 5000
HEALTHCHECK --interval=10s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -fsS "http://localhost:${PORT}/readyz" || exit 1
CMD ["sidereal-server"]
