# 🌌 Sidereal: Self-Hosted Astrophotography Management System

## 1. 🌟 Introduction

**Sidereal** is a **self-hosted, privacy-focused photo gallery and asset management system** designed specifically for astrophotographers. It addresses the critical need for specialized metadata management, intelligent celestial object identification, and equipment tracking—all missing in general-purpose photo management tools.

The primary goal is to provide a single, secure platform for astrophotographers to store their images and automatically enrich them with the highly technical, scientific metadata required for the hobby, without sacrificing data ownership or privacy.

## 2. 🚀 Quick Start

### Prerequisites
- **Node.js** >= 24.0.0 (with native TypeScript support)
- **pnpm** >= 9.0.0
- **Docker** and **Docker Compose**

### Getting Started

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd sidereal
   ```

2. **Set up environment variables**
   ```bash
   cp .env.example .env
   ```

3. **Start all services**
   ```bash
   docker compose up
   ```

4. **Access the application**
   - Frontend: http://localhost:3000
   - Backend API: http://localhost:3001
   - Database Admin (Adminer): http://localhost:8080

## 3. 🏗️ Technology Stack

### Monorepo Structure
- **Package Manager**: pnpm workspaces
- **Build System**: Vite (frontend), TypeScript (both)

### Frontend
- **Framework**: React 19
- **Server**: Hono
- **Build Tool**: Vite
- **Styling**: Tailwind CSS
- **Testing**: Vitest + Playwright

### Backend
- **Framework**: Hono (Node.js API)
- **Database**: PostgreSQL 16
- **ORM**: Drizzle ORM
- **Runtime**: Node 24 with native TypeScript support

### Infrastructure
- **Containerization**: Docker Compose
- **Development**: Hot-reload for frontend and backend
- **Database UI**: Adminer

## 4. 📁 Project Structure

```
sidereal/
├── apps/
│   ├── frontend/          # React + Vite frontend application
│   └── backend/           # Hono API backend application
├── packages/
│   ├── typescript-config/ # Shared TypeScript configurations
│   ├── eslint-config/     # Shared ESLint rules
│   └── shared-types/      # Common TypeScript types
├── docs/
│   └── planning/          # Project vision, milestones, and requirements
├── docker-compose.yml     # Multi-service container orchestration
└── pnpm-workspace.yaml    # Monorepo workspace configuration
```

## 5. 📚 Development Documentation

- **Frontend**: See [apps/frontend/README.md](apps/frontend/README.md)
- **Backend**: See [apps/backend/README.md](apps/backend/README.md)

## 6. 🧪 Testing

```bash
# Run all tests
pnpm test

# Frontend unit tests
cd apps/frontend && pnpm test

# Frontend E2E tests
cd apps/frontend && pnpm test:e2e

# Backend tests
cd apps/backend && pnpm test
```