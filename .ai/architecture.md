# Architecture

This document outlines the technical architecture choices for the Sidereal project.

## Overview

- **Model**: Three-tier web application.
- **Communication**: The React web (frontend) interacts with the Hono backend (server) via API calls (e.g., RESTful APIs) to fetch and send data.  The Hono backend (server) processes these requests, queries the PostgreSQL database for data, and returns a response to the web frontend.
- **Deployment**: The entire stack (web, backend, and database) is managed and deployed using Docker and Docker Compose, ensuring a consistent and reproducible environment.

## Tech Stack

### Web Frontend
- **Framework**: React for building the user interface
- **CSS Framework**: Tailwind CSS for utility-first styling
- **UI Library**: shadcn/ui for pre-built, accessible components

### Backend (API)
- **Web Framework**: Hono (a lightweight, fast web framework, often used for APIs on edge runtimes or Node.js)
- **ORM/Query Builder**: Drizzle
- **Database**: PostgreSQL

### Testing and Quality
- **Unit and Integration Test Framework**: Jest
- **End-to-end Test Framework**: Playwright
- **Linting/Formatting**: ESLint
