# Spec Requirements: Initial Project Setup

## Initial Description
Initial project setup and core foundation for Sidereal.

## Requirements Discussion

### First Round Questions

**Q1: Docker Compose Configuration**
I assume we should include the complete Docker Compose configuration with all three containers: frontend (Hono + React), backend (Hono API), and PostgreSQL database. Should all components be part of the initial setup, or would you prefer to start with a subset?

**Answer:** Yes, include the complete Docker Compose configuration with all three containers: frontend (Hono + React), backend (Hono API), and PostgreSQL database. All components should be part of the initial setup.

**Q2: Development Environment**
I'm thinking we should include hot-reload capabilities for both frontend and backend, with volume mounts for database persistence. Should we also include a database admin UI (like Adminer or pgAdmin) for development convenience?

**Answer:** Yes, include hot-reload capabilities for both frontend and backend with volume mounts for database persistence. Also include a database admin UI (Adminer preferred) for development convenience.

**Q3: Project Structure**
I assume we'll follow a Turborepo-style monorepo pattern with `/apps/frontend`, `/apps/backend`, `/packages` for shared code, and `/docker-compose.yml` at the root. Is this structure correct, or would you prefer a different organization?

**Answer:** Follow Turborepo-style monorepo pattern with:
- `/apps/frontend` - Hono + React + Tailwind + shadcn/ui application
- `/apps/backend` - Hono + Drizzle ORM API application
- `/packages` - Shared code between apps (UI components, utilities, TypeScript configs, ESLint configs, etc.)
- `/docker-compose.yml` at root
- Root-level configuration (package.json with workspaces, turbo.json if using Turborepo)

**Q4: Initial Dependencies**
For the frontend, I'm thinking: Hono, React, Tailwind CSS, shadcn/ui, Vite, and TypeScript. For the backend: Hono, Drizzle ORM, node-postgres, and TypeScript. Should we also include testing frameworks (like Vitest, Playwright) from the start?

**Answer:**
- Frontend (apps/frontend): Hono, React, Tailwind CSS, shadcn/ui, Vite, TypeScript
- Backend (apps/backend): Hono, Drizzle ORM, node-postgres, TypeScript
- Shared packages: TypeScript config, ESLint config, shared types
- Testing: Vitest, Playwright, ESLint
- Monorepo tooling: npm workspaces or Turborepo (to be decided in spec)

**Q5: Database Schema**
Should we include Drizzle database schema initialization and migration system fully configured, or just the basic connection setup with migrations to be added later?

**Answer:** Yes, include Drizzle database schema initialization and migration system fully configured:
- Drizzle configuration files in apps/backend
- Initial migration setup
- Database connection configuration
- Migration scripts in package.json

**Q6: Environment Configuration**
I assume we should include `.env.example` files with defaults for database credentials (POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB), ports (FRONTEND_PORT, BACKEND_PORT), and environment (NODE_ENV). Is that sufficient, or are there other environment variables needed?

**Answer:** Include .env.example files with defaults for:
- Database: POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB, DATABASE_URL
- Ports: FRONTEND_PORT, BACKEND_PORT
- Environment: NODE_ENV

**Q7: CI/CD Pipeline**
Should we include basic GitHub Actions workflows for testing and linting in this initial setup, or defer CI/CD configuration to a later phase?

**Answer:** Out of scope - focus on local development environment.

**Q8: Exclusions**
What should explicitly NOT be included in this initial setup? For example: authentication/authorization, domain-specific models (Equipment, Images, Metadata), production deployment configs, or extensive UI beyond a health check page?

**Answer:** Exclusions:
- Authentication/authorization
- Domain-specific models (Equipment, Images, Metadata)
- Production deployment configs
- UI beyond health check/status pages
- API endpoints beyond health checks
- Comprehensive tests (infrastructure only)
- Production optimizations
- Third-party integrations
- Monitoring/logging systems

### Existing Code to Reference

No similar existing features identified for reference. This is a greenfield project - only `docs/planning` documentation exists.

### Follow-up Questions

**Follow-up 1: Standards Alignment**
Review the project standards in `docs/planning/architecture.md` to ensure requirements align with coding style, testing approach, validation requirements, and frontend/backend patterns. Should the spec explicitly reference these standards?

**Answer:** Yes, document all findings in `planning/requirements.md` following the standard format. Include sections for each key area: monorepo structure (apps/ and packages/), Docker setup, dependencies, database configuration, development workflow, and explicit scope boundaries. Ensure alignment with `docs/planning/architecture.md` for:
- Coding style (naming, formatting, functions)
- Testing (minimal during dev, defer edge cases)
- Validation (server-side required)
- Backend standards (API, models, queries, migrations)
- Frontend standards (components, CSS, accessibility, responsive)

## Visual Assets

### Files Provided:
No visual assets provided.

### Visual Insights:
Not applicable - this spec focuses on project structure and technical stack setup, not UI design.

## Requirements Summary

### Functional Requirements

**Monorepo Structure:**
- Turborepo-style monorepo with npm workspaces (or Turborepo)
- `/apps/frontend` - Hono + React + Tailwind + shadcn/ui + Vite application
- `/apps/backend` - Hono + Drizzle ORM API application
- `/packages` - Shared code (UI components, utilities, TypeScript configs, ESLint configs, shared types)
- Root-level configuration files (package.json, turbo.json, docker-compose.yml)

**Docker Environment:**
- Three containers: frontend, backend, PostgreSQL database
- Hot-reload for both frontend and backend
- Volume mounts for database persistence
- Adminer for database administration UI
- All containers orchestrated via docker-compose.yml

**Frontend Application (apps/frontend):**
- Hono server with React
- Vite build tool
- Tailwind CSS for styling
- shadcn/ui component library
- TypeScript configuration
- Health check/status page only
- Hot-reload development mode

**Backend Application (apps/backend):**
- Hono API framework
- Drizzle ORM for database interactions
- node-postgres driver
- TypeScript configuration
- Health check endpoint only
- Database connection handling
- Hot-reload development mode

**Database Configuration:**
- PostgreSQL container
- Drizzle configuration files
- Initial migration system setup
- Migration scripts in package.json
- Database connection configuration
- Volume persistence for data

**Development Tooling:**
- Vitest for unit testing
- Playwright for end-to-end testing
- ESLint for code quality
- TypeScript for type safety
- Shared configs in /packages

**Environment Configuration:**
- .env.example files for both apps
- Database credentials: POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB, DATABASE_URL
- Port configuration: FRONTEND_PORT, BACKEND_PORT
- Environment flag: NODE_ENV

**Development Workflow:**
- Single command to start all services (docker compose up)
- Automatic code reload on changes
- Database admin UI accessible via browser
- Shared package management across apps
- Migration scripts for database changes

### Reusability Opportunities

This is a greenfield project with no existing codebase to reuse. However, the setup should create reusable patterns for:
- Shared TypeScript configurations
- Shared ESLint configurations
- Shared UI components in /packages
- Shared types and utilities
- Standardized Docker development environment

### Scope Boundaries

**In Scope:**
- Complete monorepo structure with apps/ and packages/ directories
- Docker Compose configuration with all three services
- Frontend application scaffold with Hono + React + Tailwind + shadcn/ui
- Backend application scaffold with Hono + Drizzle ORM
- PostgreSQL database container with Adminer UI
- Drizzle migration system fully configured
- Environment configuration files (.env.example)
- Hot-reload development environment
- Basic health check endpoints and pages
- Testing framework setup (Vitest, Playwright, ESLint)
- Shared package configurations (TypeScript, ESLint)
- Development workflow documentation (README files)
- Monorepo tooling (npm workspaces or Turborepo)

**Out of Scope:**
- Authentication and authorization systems
- Domain-specific models (Equipment, Images, Metadata)
- Production deployment configurations
- Comprehensive UI beyond health check/status pages
- API endpoints beyond health checks
- Comprehensive test suites (only infrastructure setup)
- Production optimizations (caching, CDN, etc.)
- Third-party service integrations
- Monitoring and logging systems
- CI/CD pipelines (GitHub Actions, etc.)
- Domain-specific business logic
- User management features
- Data seeding scripts
- Backup and recovery systems

### Technical Considerations

**Alignment with Project Standards:**
The specification must ensure alignment with standards defined in `docs/planning/architecture.md`:
- Coding style: naming conventions, formatting, function structure
- Testing: minimal during development, defer edge cases, focus on critical paths
- Validation: server-side validation required for all inputs
- Backend: API design, model structure, query patterns, migration practices
- Frontend: component architecture, CSS methodology, accessibility, responsive design

**Technology Stack:**
- Frontend Framework: Hono + React
- Backend Framework: Hono
- Database: PostgreSQL
- ORM: Drizzle
- Styling: Tailwind CSS + shadcn/ui
- Build Tool: Vite
- Language: TypeScript
- Testing: Vitest (unit), Playwright (e2e)
- Linting: ESLint
- Monorepo: npm workspaces or Turborepo (to be decided)
- Containerization: Docker + Docker Compose
- Database Admin: Adminer

**Integration Points:**
- Frontend communicates with backend API over HTTP
- Backend connects to PostgreSQL database via Drizzle ORM
- All services networked via Docker Compose
- Shared packages imported from /packages directory
- Environment variables control service configuration

**Development Constraints:**
- Must work with Docker Compose for local development
- Must support hot-reload for rapid iteration
- Must persist database data across container restarts
- Must follow monorepo patterns for code sharing
- Must use TypeScript throughout the codebase
- Must include database migration tooling from the start

**Decision Points for Spec Writer:**
- Choose between npm workspaces or Turborepo for monorepo management
- Define exact port assignments for each service
- Determine initial file structure within apps/frontend and apps/backend
- Specify exact versions for critical dependencies
- Define health check endpoint/page implementations
- Establish naming conventions for Docker services and volumes
