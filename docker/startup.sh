#!/bin/sh

# Sidereal Container Startup Script
set -e

mask_connection_string() {
    echo "$1" | sed 's|://[^:]*:[^@]*@|://****:****@|'
}

run_auto_db_migration() {
    if [ -z "$AUTO_DB_MIGRATE_FROM" ]; then
        return
    fi

    MIGRATE_SCRIPT="/app/dist/tools/scripts/migrate-db.js"
    if [ ! -f "$MIGRATE_SCRIPT" ]; then
        echo "Automatic database migration requested, but $MIGRATE_SCRIPT was not found."
        exit 1
    fi

    AUTO_DB_MIGRATE_MARKER=${AUTO_DB_MIGRATE_MARKER:-/app/config/.auto-db-migrated}
    AUTO_DB_MIGRATE_ONCE=${AUTO_DB_MIGRATE_ONCE:-true}

    if [ "$AUTO_DB_MIGRATE_ONCE" = "true" ] && [ -f "$AUTO_DB_MIGRATE_MARKER" ]; then
        echo "Automatic database migration already completed (marker $AUTO_DB_MIGRATE_MARKER exists); skipping."
        return
    fi

    if [ -n "$AUTO_DB_MIGRATE_TO" ]; then
        AUTO_DB_MIGRATE_TARGET="$AUTO_DB_MIGRATE_TO"
    elif [ -n "$DATABASE_URL" ]; then
        AUTO_DB_MIGRATE_TARGET="$DATABASE_URL"
    else
        SQLITE_TARGET_PATH=${SQLITE_DB_PATH:-/app/config/sidereal.db}
        AUTO_DB_MIGRATE_TARGET="sqlite:$SQLITE_TARGET_PATH"
    fi

    if [ "${AUTO_DB_MIGRATE_RESET_SQLITE:-true}" = "true" ] && [ "${AUTO_DB_MIGRATE_TARGET#sqlite:}" != "$AUTO_DB_MIGRATE_TARGET" ]; then
        SQLITE_TARGET_PATH="${AUTO_DB_MIGRATE_TARGET#sqlite:}"
        echo "  Removing existing SQLite database at $SQLITE_TARGET_PATH before migration..."
        rm -f "$SQLITE_TARGET_PATH"
    fi

    echo "Running automatic database migration..."
    echo "  From: $(mask_connection_string "$AUTO_DB_MIGRATE_FROM")"
    echo "  To: $(mask_connection_string "$AUTO_DB_MIGRATE_TARGET")"

    if ! su-exec sidereal node "$MIGRATE_SCRIPT" --from "$AUTO_DB_MIGRATE_FROM" --to "$AUTO_DB_MIGRATE_TARGET"; then
        echo "Automatic database migration failed; aborting startup."
        exit 1
    fi

    if [ "$AUTO_DB_MIGRATE_ONCE" = "true" ]; then
        mkdir -p "$(dirname "$AUTO_DB_MIGRATE_MARKER")"
        touch "$AUTO_DB_MIGRATE_MARKER"
    fi

    echo "Automatic database migration completed successfully."
}

# Handle PUID and PGID for proper volume permissions
PUID=${PUID:-1001}
PGID=${PGID:-1001}

echo "Updating sidereal user to PUID=$PUID and GID=$PGID..."
groupmod -o -g "$PGID" nodejs || true
usermod -o -u "$PUID" sidereal || true

# Ensure proper ownership of application and data directories
echo "Setting directory permissions..."
chown -R sidereal:nodejs /app/config /app/logs /app/sidecars /app/cache /app/dist /app/data

echo "Starting Sidereal container..."
echo "Node.js version: $(node --version)"
echo "Environment: ${NODE_ENV:-development}"
if [ -n "$DATABASE_URL" ]; then
    echo "Database: PostgreSQL ($(echo "$DATABASE_URL" | sed 's|://[^:]*:[^@]*@|://****:****@|'))"
else
    echo "Database: SQLite (built-in at /app/config/sidereal.db)"
fi
echo "Plate solving enabled: ${ENABLE_PLATE_SOLVING:-true}"
echo "XMP sidecar path: ${XMP_SIDECAR_PATH:-/app/sidecars}"

# Wait for database to be ready
if [ -n "$DATABASE_URL" ]; then
    echo "Waiting for database connection..."
    # Extract host and port from DATABASE_URL
    DB_HOST=$(echo $DATABASE_URL | sed 's/.*@\([^:]*\):.*/\1/')
    DB_PORT=$(echo $DATABASE_URL | sed 's/.*:\([0-9]*\)\/.*/\1/')
    
    # Wait for PostgreSQL to be ready
    timeout=60
    while [ $timeout -gt 0 ]; do
        if nc -z "$DB_HOST" "$DB_PORT" 2>/dev/null; then
            echo "Database is ready!"
            break
        fi
        echo "Waiting for database... ($timeout seconds remaining)"
        sleep 2
        timeout=$((timeout - 2))
    done
    
    if [ $timeout -le 0 ]; then
        echo "ERROR: Database is not responding after 60 seconds"
        exit 1
    fi
else
    echo "No DATABASE_URL provided, using built-in SQLite database"
fi

# Run database migrations as sidereal user
echo "Running database migrations..."
if [ -n "$DATABASE_URL" ]; then
    # PostgreSQL migrations
    if [ -f "/app/dist/tools/scripts/apply-pg-migrations.js" ]; then
        echo "Running PostgreSQL migrations..."
        su-exec sidereal node /app/dist/tools/scripts/apply-pg-migrations.js || {
            echo "PostgreSQL migration failed, but continuing..."
        }
    else
        echo "PostgreSQL migration script not found"
    fi
fi

run_auto_db_migration

# Set up signal handlers for graceful shutdown
cleanup() {
    echo "Received shutdown signal, stopping processes..."
    
    # Kill worker process if running
    if [ -n "$WORKER_PID" ] && kill -0 "$WORKER_PID" 2>/dev/null; then
        echo "Stopping worker process (PID: $WORKER_PID)..."
        kill -TERM "$WORKER_PID"
        wait "$WORKER_PID" 2>/dev/null || true
    fi
    
    # Kill main process if running
    if [ -n "$MAIN_PID" ] && kill -0 "$MAIN_PID" 2>/dev/null; then
        echo "Stopping main process (PID: $MAIN_PID)..."
        kill -TERM "$MAIN_PID"
        wait "$MAIN_PID" 2>/dev/null || true
    fi
    
    echo "Shutdown complete"
    exit 0
}

# Trap signals for graceful shutdown
trap cleanup TERM INT

# Start main application as sidereal user
echo "Starting main application..."
su-exec sidereal node /app/dist/index.js &
MAIN_PID=$!
echo "Main process started with PID: $MAIN_PID"

# Start worker process if enabled as sidereal user
if [ "${ENABLE_PLATE_SOLVING:-true}" = "true" ]; then
    echo "Starting worker process..."
    su-exec sidereal node /app/dist/worker.js &
    WORKER_PID=$!
    echo "Worker process started with PID: $WORKER_PID"
else
    echo "Worker process disabled (ENABLE_PLATE_SOLVING=${ENABLE_PLATE_SOLVING})"
    WORKER_PID=""
fi

# Function to check if processes are running
check_processes() {
    # Check main process
    if ! kill -0 "$MAIN_PID" 2>/dev/null; then
        echo "ERROR: Main process (PID: $MAIN_PID) has died"
        return 1
    fi
    
    # Check worker process if it should be running
    if [ "${ENABLE_PLATE_SOLVING:-true}" = "true" ] && [ -n "$WORKER_PID" ]; then
        if ! kill -0 "$WORKER_PID" 2>/dev/null; then
            echo "WARNING: Worker process (PID: $WORKER_PID) has died, restarting..."
            su-exec sidereal node /app/dist/worker.js &
            WORKER_PID=$!
            echo "Worker process restarted with PID: $WORKER_PID"
        fi
    fi
    
    return 0
}

# Monitor processes
echo "Application started successfully. Monitoring processes..."
while true; do
    if ! check_processes; then
        echo "Critical process failure, shutting down container"
        cleanup
        exit 1
    fi
    sleep 30
done
