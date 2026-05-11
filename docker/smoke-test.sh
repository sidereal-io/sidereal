#!/bin/bash
# Smoke test: starts Postgres + the app image, verifies /api/health responds.
# Usage: ./docker/smoke-test.sh <image-name>
# Example: ./docker/smoke-test.sh sidereal:test-123

set -euo pipefail

IMAGE="${1:?Usage: $0 <image-name>}"
NETWORK="smoke-test-net"
DB_CONTAINER="smoke-test-db"
APP_CONTAINER="smoke-test-app"

cleanup() {
  docker stop "$APP_CONTAINER" "$DB_CONTAINER" 2>/dev/null || true
  docker rm "$APP_CONTAINER" "$DB_CONTAINER" 2>/dev/null || true
  docker network rm "$NETWORK" 2>/dev/null || true
}
trap cleanup EXIT

# Start Postgres
docker network create "$NETWORK"
docker run -d --name "$DB_CONTAINER" --network "$NETWORK" \
  -e POSTGRES_DB=sidereal -e POSTGRES_USER=sidereal -e POSTGRES_PASSWORD=testpass \
  postgres:15-alpine

echo "Waiting for Postgres..."
for i in $(seq 1 15); do
  docker exec "$DB_CONTAINER" pg_isready -U sidereal -d sidereal > /dev/null 2>&1 && break
  sleep 2
done

# Start the app
docker run -d --name "$APP_CONTAINER" --network "$NETWORK" -p 5000:5000 \
  -e NODE_ENV=production \
  -e DATABASE_URL=postgresql://sidereal:testpass@${DB_CONTAINER}:5432/sidereal \
  "$IMAGE"

# Wait for health endpoint
echo "Waiting for health check..."
for i in $(seq 1 30); do
  if curl -sf http://localhost:5000/api/health > /dev/null 2>&1; then
    echo "Health check passed"
    exit 0
  fi
  sleep 2
done

echo "Health check failed after 60s"
docker logs "$APP_CONTAINER"
exit 1
