# Architecture

This document describes the system architecture and technology choices for the Sidereal project. It is intended for developers who need to understand the system design and implementation details.

## System Architecture

Sidereal follows a three-tier web application architecture with clear separation of concerns between presentation, business logic, and data persistence.

### Component Overview

#### Web Frontend
**Purpose**: Provides the user interface and handles all user interactions.

**Responsibilities**:
- Renders UI components and manages client-side state
- Handles user input validation and feedback
- Makes HTTP requests to the backend API
- Manages client-side routing and navigation

**Boundaries**: The frontend has no direct database access. All data operations must go through the backend API.

#### Backend API
**Purpose**: Serves as the application server, handling business logic and data operations.

**Responsibilities**:
- Exposes HTTP endpoints for frontend consumption
- Implements business logic and validation rules
- Manages database queries and transactions
- Handles authentication and authorization
- Transforms and formats data for API responses

**Boundaries**: The backend is the only component with database access. It does not serve static files or handle UI rendering.

#### Database
**Purpose**: Provides persistent data storage for the application.

**Responsibilities**:
- Stores application data in a relational schema
- Enforces data integrity through constraints
- Manages transactions and concurrent access

**Boundaries**: The database is only accessible by the backend API. No direct client connections are permitted.

## Data Flow

The typical data flow through the system follows this pattern:

1. **User Action**: User interacts with the web frontend (e.g., submits a form, clicks a button)
2. **API Request**: Frontend sends an HTTP request to a backend API endpoint
3. **Business Logic**: Backend validates the request and executes business logic
4. **Database Query**: Backend uses the ORM to query or modify data in PostgreSQL
5. **Response**: Backend formats the data and returns an HTTP response
6. **UI Update**: Frontend receives the response and updates the user interface

**Error Handling**: Errors at any stage are propagated back to the frontend with appropriate HTTP status codes and error messages.

## Deployment & Environment

### Containerization
The entire stack is containerized using Docker, with each component running in its own container:
- **Frontend Container**: Serves the built React application
- **Backend Container**: Runs the Hono API server
- **Database Container**: Runs PostgreSQL

### Orchestration
Docker Compose manages the multi-container setup, defining:
- Service dependencies (backend depends on database, frontend depends on backend)
- Network configuration (containers communicate via Docker network)
- Volume mounts (database persistence, development hot-reload)
- Environment variables (connection strings, ports, configuration)

### Communication
- Frontend and backend communicate over HTTP
- Backend and database communicate via PostgreSQL wire protocol
- All inter-container communication happens on a private Docker network

## Technology Stack

### Web Frontend
- **Framework**: Hono
- **UI**: React
- **CSS**: Tailwind CSS
- **Commponents**: shadcn/ui

### Backend API
- **Framework**: Hono
- **ORM/Query Builder**: Drizzle
- **Runtime**: Node.js

### Database
- **RDBMS**: PostgreSQL - Relational database with ACID compliance

## Testing & Quality

### Testing Strategy
- **Unit & Integration Tests**: Vitest - Fast unit test framework with built-in mocking
- **End-to-End Tests**: Playwright - Browser automation for full-stack testing
- **Code Quality**: ESLint - Static analysis and code style enforcement

### Test Coverage
- Frontend: Component tests, integration tests for API interactions
- Backend: Unit tests for business logic, integration tests for API endpoints
- E2E: Critical user workflows across the entire stack
