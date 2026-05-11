# Multi-stage build for Sidereal
# SECURITY: This image does not contain any secrets or API keys
# All sensitive configuration is provided via environment variables at runtime
FROM node:24-alpine AS builder

# Upgrade npm and patch its bundled dependencies to fix known CVEs
RUN npm install -g npm@11.14.1 && \
    npm pack minimatch@10.2.4 && \
    tar -xzf minimatch-10.2.4.tgz -C /usr/local/lib/node_modules/npm/node_modules/minimatch --strip-components=1 && \
    rm minimatch-10.2.4.tgz && \
    npm pack tar@7.5.11 && \
    tar -xzf tar-7.5.11.tgz -C /usr/local/lib/node_modules/npm/node_modules/tar --strip-components=1 && \
    rm tar-7.5.11.tgz && \
    npm cache clean --force

# Set working directory
WORKDIR /build

# Copy package files
COPY package*.json ./
COPY tsconfig.json ./
COPY vite.config.ts ./
COPY tailwind.config.ts ./
COPY postcss.config.js ./
COPY drizzle.config.ts ./

# Install all dependencies (including dev dependencies for build)
# Handle npm bug with Rollup optional dependencies on ARM64/musl
RUN npm ci || (npm cache clean --force && npm install)

# Copy source code
COPY apps/ ./apps/
COPY packages/ ./packages/
COPY tools/ ./tools/

# Build frontend and copy assets
RUN npm run build:docker

# Production stage
FROM node:24-alpine AS runtime

# Upgrade npm and patch its bundled dependencies to fix known CVEs
RUN npm install -g npm@11.14.1 && \
    npm pack minimatch@10.2.4 && \
    tar -xzf minimatch-10.2.4.tgz -C /usr/local/lib/node_modules/npm/node_modules/minimatch --strip-components=1 && \
    rm minimatch-10.2.4.tgz && \
    npm pack tar@7.5.11 && \
    tar -xzf tar-7.5.11.tgz -C /usr/local/lib/node_modules/npm/node_modules/tar --strip-components=1 && \
    rm tar-7.5.11.tgz && \
    npm cache clean --force

# Upgrade base packages (fixes zlib CVEs) and install runtime dependencies
# hadolint ignore=DL3018
RUN apk upgrade --no-cache && \
    apk add --no-cache curl su-exec shadow

# Create app user for security (will be remapped via PUID/PGID if provided)
RUN addgroup -g 1001 -S nodejs && \
    adduser -S sidereal -u 1001 -G nodejs

# Set working directory
WORKDIR /app

# Copy package files
COPY package*.json ./

# Install production dependencies (including better-sqlite3 for built-in SQLite support)
RUN apk add --no-cache --virtual .build-deps python3 make g++ && \
    npm ci --omit=dev && \
    apk del .build-deps && \
    npm cache clean --force

# Copy built application from builder stage (includes tools, config, and public assets)
COPY --from=builder /build/dist ./dist

# Create directories for runtime
RUN mkdir -p /app/config /app/logs /app/sidecars /app/cache/thumbnails

# Copy startup script
COPY docker/startup.sh ./
RUN chmod +x startup.sh

# Run as root initially to allow PUID/PGID remapping in startup.sh
# hadolint ignore=DL3002
USER root

# Expose port
EXPOSE 5000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
  CMD curl -f http://localhost:5000/api/health || exit 1

# Start application
CMD ["./startup.sh"]
