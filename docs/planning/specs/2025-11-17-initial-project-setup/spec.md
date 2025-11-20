# Specification: Initial Project Setup and Core Foundation

## Goal
Establish a complete, containerized monorepo development environment with Hono + React frontend, Hono API backend, PostgreSQL database, and full tooling infrastructure ready for building the Equipment Catalog feature.

## User Stories
- As a developer, I want to run `docker compose up` and have all services (frontend, backend, database) start with hot-reload so I can begin development immediately
- As a developer, I want a fully configured database migration system so I can evolve the schema safely as features are added
- As a developer, I want shared TypeScript and ESLint configurations across apps so the codebase maintains consistent code quality and style

## Specific Requirements

**Monorepo Structure**
- Create Turborepo-style monorepo with `/apps/frontend`, `/apps/backend`, and `/packages` directories
- Configure npm workspaces (or Turborepo) for cross-workspace dependencies and shared package management
- Place root-level `package.json` with workspace definitions, `turbo.json` (if using Turborepo), and `docker-compose.yml`
- Include `.gitignore` to exclude `node_modules`, `.env`, Docker volumes, and build artifacts
- Create README files at root and in each app explaining project structure and setup commands

**Docker Compose Configuration**
- Define four services: frontend (Hono + React), backend (Hono API), PostgreSQL database, and Adminer UI
- Configure volume mounts for frontend/backend code to enable hot-reload during development
- Create named volume for PostgreSQL data persistence across container restarts
- Set up Docker network for service communication with appropriate container names and hostnames
- Map ports for frontend (3000), backend (3001), database (5432), and Adminer (8080)
- Include healthcheck configurations for each service to verify readiness
- Configure service dependencies to ensure database starts before backend

**Frontend Application (apps/frontend)**
- Initialize Hono server with React integration using Vite as build tool
- Configure Tailwind CSS with shadcn/ui component library including initialization script
- Create TypeScript configuration extending shared config from `/packages/typescript-config`
- Implement simple health check/status page showing "Sidereal Frontend" and connection status to backend
- Set up hot-reload development mode with Vite dev server
- Include `package.json` with scripts for dev, build, and type-check
- Create Dockerfile for frontend service with development dependencies and volume mount setup

**Backend Application (apps/backend)**
- Initialize Hono API framework with TypeScript configuration
- Configure Drizzle ORM with node-postgres driver for PostgreSQL connection
- Create health check endpoint at GET `/health` returning status and database connection state
- Set up hot-reload development mode using tsx or nodemon for TypeScript execution
- Include `package.json` with scripts for dev, build, migration commands, and type-check
- Create Dockerfile for backend service with development dependencies and volume mount setup
- Configure CORS middleware to allow frontend origin during development

**Database Setup and Migrations**
- Configure PostgreSQL container with environment variables for user, password, and database name
- Create Drizzle configuration file (`drizzle.config.ts`) pointing to PostgreSQL connection
- Set up migration directory structure in `apps/backend/drizzle` for schema files
- Implement initial migration with minimal schema (single health_check table for connection verification)
- Add migration scripts in backend `package.json`: generate, run, drop, and push commands
- Include database connection utility with error handling and retry logic
- Configure Adminer container for web-based database administration UI

**Shared Package Configurations**
- Create `packages/typescript-config` with base tsconfig.json for strict type checking and modern ES modules
- Create `packages/eslint-config` with shared ESLint rules aligned with coding-style.md standards
- Create `packages/shared-types` for common TypeScript types used across frontend and backend
- Include package.json files in each shared package with proper exports and dependencies
- Configure workspace references so apps can import from shared packages using workspace protocol

**Testing Infrastructure**
- Install Vitest in both frontend and backend with shared configuration from packages
- Install Playwright in frontend for end-to-end testing with browser automation
- Configure ESLint with TypeScript parser for both apps extending shared config
- Create minimal test examples demonstrating infrastructure works (not comprehensive test suites)
- Add test scripts to package.json files for running unit and e2e tests
- Configure test reporters for clear output in terminal during development

**Environment Configuration**
- Create `.env.example` in project root with DATABASE_URL, POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB, NODE_ENV, FRONTEND_PORT, BACKEND_PORT
- Create `.env.example` files in both apps showing app-specific environment variables
- Document environment variable purposes and default values in root README
- Add `.env` to `.gitignore` to prevent committing secrets
- Configure Docker Compose to use environment variables from .env file for service configuration

**Development Workflow**
- Single command `docker compose up` starts all services with proper dependencies and healthchecks
- Frontend and backend auto-reload on file changes via volume mounts and dev servers
- Database data persists across container restarts via named volume
- Adminer accessible at http://localhost:8080 for visual database inspection
- Clear error messages when services fail healthchecks or lose connections
- Migration workflow: generate migration, review SQL, run migration, verify in Adminer

**Project Documentation**
- Root README with project overview, technology stack, and quick start instructions (clone, copy .env.example, docker compose up)
- Frontend README explaining app structure, available scripts, and how to add components
- Backend README explaining API structure, migration workflow, and how to add endpoints
- Include architecture diagram showing service communication flow between containers

## Visual Design
No visual assets provided for this infrastructure setup specification.

## Existing Code to Leverage

**No existing application code**
- This is a greenfield project with only `docs/planning` documentation currently in the repository
- No existing patterns to follow or components to reuse
- Future features will leverage the infrastructure created by this specification
- Shared packages created here will become the foundation for code reuse across apps

## Out of Scope
- Authentication and authorization systems (no login, sessions, JWT, OAuth)
- Domain-specific database models for Equipment, Images, or Metadata entities
- Production deployment configurations (Kubernetes, cloud hosting, CI/CD pipelines)
- Comprehensive UI beyond health check/status page (no navigation, layouts, or feature pages)
- API endpoints beyond health checks (no CRUD operations or business logic endpoints)
- Extensive test suites covering edge cases (only infrastructure verification tests)
- Production optimizations like caching, CDN integration, or performance monitoring
- Third-party service integrations (no cloud storage, email, analytics, or external APIs)
- Monitoring, logging, and observability systems (no Sentry, DataDog, or log aggregation)
- Data seeding scripts or fixture generation for development databases
