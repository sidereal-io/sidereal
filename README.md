<p align="center">
  <img src="apps/client/public/logo.png" width="300">
  <h1 style="font-size: 55px" align="center">SIDEREAL</h1>
</p>

<p align="center">
  <a href="./docker"><img src="https://img.shields.io/badge/Docker-Ready-blue?logo=docker"/></a>
  <a href="./LICENSE"><img src="https://img.shields.io/badge/License-MIT-green.svg" />
  <a href="./docker/unraid-templates"><img src="https://img.shields.io/badge/UnRAID-Compatible-orange)" /></a>
  <a href="https://discord.gg/ffZ8cuJ8Kh"><img src="https://img.shields.io/badge/Discord-Join%20Chat-5865F2?logo=discord&logoColor=white" alt="Join us on Discord" /></a>
</p>

<p align="center">
  <a href="https://github.com/mstelz/Sidereal/actions/workflows/docker-build-push.yml"><img src="https://github.com/mstelz/Sidereal/actions/workflows/docker-build-push.yml/badge.svg" alt="Build Status" /></a>
  <a href="https://github.com/mstelz/Sidereal/actions/workflows/release.yml"><img src="https://github.com/mstelz/Sidereal/actions/workflows/release.yml/badge.svg" alt="Release" /></a>
  <a href="https://github.com/mstelz/Sidereal/security"><img src="https://img.shields.io/badge/Security-Trivy%20Scanned-brightgreen" alt="Security Scan" /></a>
</p>

**Sidereal** is a self-hosted photo gallery and management system designed specifically for astrophotographers. Built to integrate seamlessly with your [Immich](https://immich.app/) photo library, it provides intelligent plate solving, equipment tracking, and comprehensive metadata management tailored for deep-sky imaging workflows.

Perfect for organizing, analyzing, and showcasing your astrophotography collection with full control over your data and infrastructure.

> **Disclaimer**: Sidereal is an independent project and is not affiliated with, endorsed by, or officially connected to Immich or its developers. Sidereal is a third-party application that integrates with Immich's public API.

<div align="center">
  <img width="80%" src="assets/images/demo.gif" />
  <br/>
  <a href="assets/images/screenshots">View all screenshots</a>
</div>

## Features

### **Self-Hosted Image Management**
- **Immich Integration**: Seamless synchronization with your self-hosted Immich photo library
- **Interactive Sky Map**: Explore your collection on a high-fidelity celestial atlas powered by Aladin Lite v3
- **Astrophotography Filtering**: Filter by telescopes, cameras, targets, constellations, and acquisition details
- **Deep Zoom Viewer**: High-resolution exploration of your deep-sky images with OpenSeaDragon
- **Metadata Preservation**: Automatic EXIF handling for astrophotography workflows
  > **Note**: XMP sidecar generation is currently experimental and may not work as intended in all configurations. Configure `IMMICH_MAPPING_PATH` and `LOCAL_MAPPING_PATH` environment variables to map Immich volume paths to local paths for sidecar file placement.
- **Zero Duplication**: View images directly from Immich without storage overhead
- **Location Editing**: Save and reuse observing sites with map picker; coordinate changes sync back to the original Immich asset automatically

### **DSO Catalog & Target Tracking**
- **Built-in OpenNGC Catalog**: Browse, search, and filter thousands of deep-sky objects by type, constellation, magnitude, and angular size
- **DSS Thumbnails**: Cached Digitized Sky Survey preview images for catalog objects
- **Target Wishlists**: Annotate catalog objects with personal notes and tags on the Targets page
- **Target Backfill**: Automatically match plate-solved images to catalog objects via the admin panel

### **Acquisition Logging**
- **Per-Filter Tracking**: Record exposure details per filter including frame count, exposure time, gain, offset, binning, and sensor temperature
- **Integration Summary**: Automatic calculation of total frames, total integration time, and filter breakdown per image

### **Plate Solving**
- **Astrometry.net Integration**: Automatic coordinate solving for your images
- **Background Processing**: Non-blocking worker processes with job queuing
- **Real-time Updates**: Live progress tracking via WebSocket connections
- **Batch Processing**: Handle multiple images simultaneously
- **Results Storage**: Persistent RA/Dec coordinates and field information

### **Astrophotography Equipment Tracking**
- **Telescope Catalog**: Manage your telescopes, mounts, and accessories with specifications
- **Camera Database**: Track sensors, filters, and imaging configurations specific to astrophotography
- **Equipment Groups**: Bundle equipment into reusable presets and apply them to images in one click
- **Session Logging**: Automatic equipment association from EXIF metadata and manual tagging

### **Admin Interface**
- **Configuration Management**: Secure API key and integration settings
- **Automated Sync**: Cron jobs automatically sync images from Immich and clean up old notifications on a configurable schedule
- **Worker Supervisor**: Background plate-solving workers automatically restart on failure with exponential backoff when Auto mode is enabled
- **Database Backup**: One-click SQLite database download from the admin panel (PostgreSQL users should use `pg_dump`)
- **Catalog Management**: Load, reload, and check for OpenNGC catalog updates; backfill target names for existing images

### **Real-Time Notifications**
- **Live Updates**: WebSocket-powered notifications for plate-solving results, Immich sync completions, and other events
- **Manual Sync**: Trigger an Immich sync at any time from the header bar
- **Notification Badge**: Unacknowledged notification count displayed in the navigation

### **Security & Deployment**
- **Docker Ready**: Multi-stage containerization with health checks
- **UnRAID Support**: Ready-to-use container templates
- **Database Options**: Built-in SQLite (default), optional PostgreSQL support

## Quick Start

> **Prerequisites**: Sidereal requires a running [Immich](https://immich.app/) instance for photo management. Ensure you have Immich set up and accessible before proceeding.

### Option 1: Docker Compose (Recommended)

Single container setup with built-in SQLite database — no external database needed:

```bash
# Download production compose file
curl -o docker-compose.prod.yml https://raw.githubusercontent.com/mstelz/Sidereal/main/docker-compose.prod.yml

# Start Sidereal
docker compose -f docker-compose.prod.yml up -d

# Access the application
open http://localhost:5000
```

**What this includes:**
- Sidereal application from GitHub Container Registry
- Built-in SQLite database with persistent storage
- Health checks and automatic restarts
- Volume mounts for configuration, logs, and database

> **PostgreSQL option**: If you prefer PostgreSQL, download [`docker-compose.postgres.yml`](docker-compose.postgres.yml) and layer it on:
> ```bash
> echo "POSTGRES_PASSWORD=your_secure_password" > .env
> docker compose -f docker-compose.prod.yml -f docker-compose.postgres.yml up -d
> ```

### Option 2: Docker Container

Single container deployment using GitHub Container Registry:

```bash
# Pull the latest image
docker pull ghcr.io/mstelz/sidereal:latest

# Run with Docker (uses built-in SQLite database)
docker run -d \
  --name sidereal \
  -p 5000:5000 \
  -e NODE_ENV=production \
  -v sidereal-config:/app/config \
  -v sidereal-cache:/app/cache \
  ghcr.io/mstelz/sidereal:latest

# Access the application
open http://localhost:5000
```

> To use PostgreSQL instead, add `-e DATABASE_URL="postgresql://user:password@host:5432/sidereal"` to the command above.

### Option 3: UnRAID Template

> **Coming Soon**: Sidereal will be available in UnRAID Community Applications for easy one-click installation.

For now, manual installation:

1. **Install Sidereal**: Add container using template URL `https://raw.githubusercontent.com/mstelz/Sidereal/main/docker/unraid-templates/sidereal.xml`
2. **Configure**: Optionally set Immich URL and API keys (can also be done via admin UI)
3. **Access**: Navigate to `http://your-server:2284`

> No external database needed — Sidereal uses a built-in SQLite database stored in the config directory. To use PostgreSQL instead, set the `DATABASE_URL` field in the template.

### Option 4: Development Setup

```bash
# Clone and install dependencies
git clone https://github.com/mstelz/Sidereal.git
cd Sidereal
npm install

# Option A: Build and run with Docker Compose (builds from source)
# Edit docker-compose.yml environment section with your settings
docker compose up -d

# Option B: Local development server
cp .env.example .env.local
# Edit .env.local with your development settings (uses SQLite by default)
npm run dev

# Option C: Worker-only deployment (standalone)
cp .env.worker.example .env.worker
# Edit .env.worker with database and API settings
npm run dev:worker:standalone

# Access at http://localhost:5000 (Docker) or http://localhost:5173 (local)
```

## Requirements

### Core Requirements
- **Immich Server**: Self-hosted photo management server (currently the only supported photo source)
- **Docker**: 20.10+ (for containerized deployment)
- **Database**: Built-in SQLite (default, no setup required). PostgreSQL 15+ optionally supported.
  - Automatic schema management with Drizzle ORM
  - Migration script included for moving data between SQLite and PostgreSQL (see [Docker docs](docker/README.md#migrating-between-sqlite-and-postgresql))

### Development Requirements
- **Node.js**: 20+ (for building from source)

### Optional Integrations
- **Astrometry.net API Key**: For automated plate solving capabilities

> **Note**: Support for additional photo sources beyond Immich is planned for future releases, but Immich is currently required as the primary photo library.

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | _(empty)_ | Optional PostgreSQL connection string. Leave empty to use built-in SQLite. |
| `PORT` | `5000` | HTTP server port |
| `NODE_ENV` | `development` | Application environment |
| `IMMICH_URL` | _(optional)_ | Immich server base URL |
| `IMMICH_API_KEY` | _(optional)_ | Immich API authentication key |
| `ASTROMETRY_API_KEY` | _(optional)_ | Astrometry.net API key |
| `ENABLE_PLATE_SOLVING` | `true` | Enable background worker |
| `PLATE_SOLVE_MAX_CONCURRENT` | `3` | Max simultaneous jobs |
| `XMP_SIDECAR_PATH` | `/app/sidecars` | Directory for XMP sidecar files |

### Admin Configuration

After startup, access the admin interface at `/admin` to configure:

- **Immich Integration**: Server URL, API keys, album picker, sync schedules
- **Astrometry Settings**: API credentials, job limits, auto-processing
- **Worker Management**: Enable/disable background processing
- **Sync Scheduling**: Automated Immich synchronization frequency

> **Tip**: Configuration via admin interface takes precedence over environment variables and persists across container restarts.

## Database Migration

Sidereal includes a bidirectional migration script for moving data between SQLite and PostgreSQL:

```bash
# PostgreSQL → SQLite
node tools/scripts/migrate-db.js \
  --from postgresql://user:pass@host:5432/sidereal \
  --to sqlite:path/to/sidereal.db

# SQLite → PostgreSQL
node tools/scripts/migrate-db.js \
  --from sqlite:path/to/local.db \
  --to postgresql://user:pass@host:5432/sidereal
```

Inside a Docker container:
```bash
node /app/dist/tools/scripts/migrate-db.js \
  --from postgresql://... \
  --to sqlite:/app/config/sidereal.db
```
To avoid running the command manually inside the container, set the `AUTO_DB_MIGRATE_FROM` environment variable (and optional `AUTO_DB_MIGRATE_TO`) on the Sidereal service. The Docker startup script runs the migration before launching the app, deletes any existing SQLite target file (unless `AUTO_DB_MIGRATE_RESET_SQLITE=false`), and writes a marker to `/app/config/.auto-db-migrated` so it only happens once unless you delete that file or set `AUTO_DB_MIGRATE_ONCE=false`.

The script handles all type conversions (timestamps, booleans, JSON, arrays) and respects foreign key ordering automatically. SQLite targets run the bundled Drizzle migrations during the copy; PostgreSQL targets should be initialized once beforehand so the schema exists.

## Container Images

Sidereal provides ready-to-use container images through GitHub Container Registry:

### Available Images
- **Latest Release**: `ghcr.io/mstelz/sidereal:latest`
- **Specific Version**: `ghcr.io/mstelz/sidereal:v1.x.x`
- **Development**: `ghcr.io/mstelz/sidereal:main`

### Supported Architectures
- `linux/amd64` (x86_64)
- `linux/arm64` (ARM64/Apple Silicon)

### Image Tags
- `latest` - Latest stable release
- `v*.*.*` - Semantic version tags (e.g., `v1.0.0`)
- `main` - Latest development build from main branch
- `sha-*` - Specific commit builds

All images are automatically built, tested, and scanned for vulnerabilities using GitHub Actions.

## Architecture

```
┌─────────────────────────────────────┐
│        Sidereal Container           │
├─────────────────────────────────────┤
│  Frontend (React + TypeScript)      │
│  ├─ Vite build system               │    ┌─────────────────────┐
│  ├─ Tailwind CSS + shadcn/ui        │    │   External APIs     │
│  └─ Real-time WebSocket client      │    ├─────────────────────┤
├─────────────────────────────────────┤◄──►│  Immich Server      │
│  Backend (Hono + Node.js)           │    │  Astrometry.net     │
│  ├─ RESTful API endpoints           │    │  Image Sources      │
│  ├─ WebSocket server (ws)           │    └─────────────────────┘
│  ├─ Image proxy & thumbnails        │
│  └─ Session management              │    ┌─────────────────────┐
├─────────────────────────────────────┤    │  PostgreSQL         │
│  Worker Manager                     │    │  (optional)         │
│  ├─ Background job processing       │◄──►│  External database  │
│  ├─ Plate solving automation        │    │  for advanced use   │
│  ├─ Crash recovery & monitoring     │    └─────────────────────┘
│  └─ Graceful shutdown handling      │
├─────────────────────────────────────┤
│  SQLite (built-in, default)         │
│  └─ /app/config/sidereal.db         │
└─────────────────────────────────────┘
```

## Project Structure

```
Sidereal/
├── apps/
│   ├── client/                 # React frontend application
│   │   ├── src/
│   │   │   ├── components/     # UI components (shadcn/ui)
│   │   │   ├── pages/          # Route components
│   │   │   ├── lib/            # Utilities and API clients
│   │   │   └── hooks/          # Custom React hooks
│   │   └── dist/               # Built frontend assets
│   └── server/                 # Node.js backend application
│       ├── src/
│       │   ├── routes/         # API route handlers
│       │   ├── services/       # Business logic services
│       │   ├── workers/        # Background job processors
│       │   └── db.ts           # Database configuration
│       └── dist/               # Compiled backend code
├── packages/
│   └── shared/                 # Shared types and schemas
│       ├── src/
│       │   ├── db/             # Database schemas (SQLite/PostgreSQL)
│       │   ├── schemas/        # Zod validation schemas
│       │   └── types/          # TypeScript type definitions
├── docker/                     # Containerization files
│   ├── Dockerfile              # Multi-stage container build
│   ├── docker-compose.yml      # Service orchestration
│   ├── startup.sh              # Container entry point
│   └── unraid-templates/       # UnRAID deployment templates
├── tools/                      # Development and migration tools
│   ├── scripts/                # Utility scripts
│   └── migrations/             # Database migration files
├── docs/                       # Documentation
└── assets/                     # Static assets and examples
```

## Development

### Prerequisites

```bash
# Install Node.js 20+
node --version  # v20+
npm --version   # 10+

# Clone repository
git clone https://github.com/mstelz/Sidereal.git
cd Sidereal
```

### Setup

```bash
# Install dependencies
npm install

# Setup environment (choose one based on your needs)
cp .env.example .env.local           # Main application settings
cp .env.worker.example .env.worker   # Worker-only deployment
# Configure your development settings

# Database schema is managed automatically on startup via Drizzle ORM
```

### Development Commands

```bash
# Start development server (SQLite database by default)
npm run dev                      # Full stack: server + frontend + worker
npm run dev:server:watch         # Server only with file watching
npm run dev:worker               # Worker process only
npm run dev:worker:standalone    # Standalone worker (uses .env.worker)

# Build for production
npm run build          # Build frontend and backend
npm run build:docker   # Build with Docker assets

# Docker development (uses SQLite by default)
docker compose up -d               # Full stack via Docker
```

### Testing

Sidereal includes comprehensive end-to-end testing using Playwright:

```bash
# Run all tests
npm run test:e2e       # Run Playwright end-to-end tests

# Interactive testing
npm run test:e2e:ui    # Run tests with Playwright UI mode
npm run test:e2e:debug # Debug tests with Playwright inspector
npm run test:e2e:headed # Run tests in headed browser mode
```

**Test Structure:**
- `tests/e2e/` - End-to-end test specifications
- `tests/e2e/pages/` - Page Object Model classes for maintainable tests
- `playwright.config.ts` - Playwright configuration with multiple browsers

**Prerequisites for Testing:**
- Sidereal application running on `http://localhost:5173`
- Test database with sample data
- All dependencies installed via `npm install`

### Code Quality

```bash
# Type checking
npm run check          # TypeScript compilation check
```

## CI/CD & Deployment

Sidereal uses GitHub Actions for continuous integration and deployment. All builds are automatically tested, scanned for vulnerabilities, and containerized.

### Automated Workflows

- **Build & Push**: Automatic Docker image builds on main branch commits
- **PR Testing**: Validates all pull requests with build tests and security scans
- **Release**: Semantic versioning releases with tagged Docker images

### Security

- All images are scanned with Trivy for vulnerabilities
- Critical vulnerabilities block deployments
- SBOM (Software Bill of Materials) generated for each build

For detailed CI/CD documentation, see [docs/CI_CD.md](docs/CI_CD.md).

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## Roadmap

### Core Features
- [ ] **Additional Photo Sources**: Support for photo libraries beyond Immich (direct uploads, other self-hosted solutions)
- [x] **Interactive Sky Map**: Celestial atlas visualization for plate-solved image collections
- [ ] Advanced image statistics and analytics for astrophotography sessions
- [ ] Equipment usage reporting and session tracking
- [x] XMP sidecar generation for astrophotography metadata
- [ ] XMP sidecar viewer and editor for astrophotography metadata

### Integrations
- [ ] **NINA Plugin**: Sync session data (targets, equipment, acquisition details) directly from N.I.N.A. to Sidereal
- [ ] **Local Plate Solving**: Integration with local solvers (ASTAP, PixInsight) for offline solving without Astrometry.net

### Advanced Features
- [ ] Advanced search and filtering with saved queries
- [ ] Bulk image processing workflows and batch operations
- [ ] View saved/tagged targets on the Sky Map
- [ ] Raw data and calibration frame management
- [ ] Mobile app companion for field use
- [ ] Community features (sharing, public galleries)

### Community & Help
- **Bug Reports**: [GitHub Issues](https://github.com/mstelz/Sidereal/issues)
- **Feature Requests**: [GitHub Discussions](https://github.com/mstelz/Sidereal/discussions)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- **[Immich](https://immich.app/)** - Inspiration and integration for photo management
- **[Astrometry.net](https://astrometry.net/)** - Plate solving service and algorithms
- **[shadcn/ui](https://ui.shadcn.com/)** - Beautiful and accessible React components
- **[Drizzle ORM](https://orm.drizzle.team/)** - Type-safe database toolkit

---

<div align="center">

**Built for the astrophotography community**

[Star this repo](https://github.com/mstelz/Sidereal) | [Report bug](https://github.com/mstelz/Sidereal/issues) | [Request feature](https://github.com/mstelz/Sidereal/discussions)

</div>
