# syntax=docker/dockerfile:1

# ---- Build stage: compile the server binary in release mode ----
FROM rust:1.85-slim-bookworm AS builder
WORKDIR /build
# Copy the whole workspace (build context is backend/) and build the server.
# The builder honors backend/rust-toolchain.toml (pinned 1.85).
COPY . .
RUN cargo build --release -p sidereal-server

# ---- Runtime stage: slim image with only the binary ----
FROM debian:bookworm-slim AS runtime
# curl backs the HEALTHCHECK; ca-certificates for future outbound TLS.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
# Unprivileged user.
RUN useradd --system --uid 10001 --user-group --no-create-home app
COPY --from=builder /build/target/release/sidereal-server /usr/local/bin/sidereal-server
USER app
ENV PORT=5000
EXPOSE 5000
HEALTHCHECK --interval=10s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -fsS "http://localhost:${PORT}/healthz" || exit 1
CMD ["sidereal-server"]
