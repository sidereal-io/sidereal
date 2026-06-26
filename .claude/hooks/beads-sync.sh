#!/bin/bash

LOCKFILE="/tmp/beads_sync_cooldown.lock"
COOLDOWN_SECONDS=45
CURRENT_TIME=$(date +%s)

pull_db() {
    # Check if we need to enforce a cooldown during active tool execution
    if [ "$ACTION" = "sync" ]; then
        if [ -f "$LOCKFILE" ] && [ $((CURRENT_TIME - $(cat "$LOCKFILE"))) -le $COOLDOWN_SECONDS ]; then
            return # Skip pulling if we checked very recently
        fi
    fi

    echo "[Beads Sync] Fetching remote database state..."
    bd dolt pull origin main --quiet 2>/dev/null || bd dolt pull origin --quiet
    echo "$CURRENT_TIME" > "$LOCKFILE"
}

push_db() {
    # 1. Commit any uncommitted mutations inside Dolt
    if bd dolt status --porcelain 2>/dev/null | grep -q '^[AMD]'; then
        echo "[Beads Sync] Local changes detected. Staging data..."
        bd dolt add -A
        bd dolt commit -m "Claude agent automatic data synchronization" --quiet
    fi

    # 2. Compare local refs to see if we need an active push to GitHub
    LOCAL_HEAD=$(bd dolt rev-parse HEAD 2>/dev/null)
    REMOTE_HEAD=$(bd dolt rev-parse origin/main 2>/dev/null || bd dolt rev-parse origin 2>/dev/null)

    if [ "$LOCAL_HEAD" != "$REMOTE_HEAD" ] && [ -n "$LOCAL_HEAD" ]; then
        echo "[Beads Sync] Pushing data changes to GitHub remote..."
        bd dolt push origin main --quiet 2>/dev/null || bd dolt push origin --quiet
        date +%s > "$LOCKFILE"
    fi
}

pull_db
push_db
