### Project Overview: Sidereal

**Sidereal** is a full-stack web application designed as a comprehensive management system for astrophotography. It allows users to organize, view, and analyze their astronomical images by integrating with external services for image storage and celestial object identification.

The application features a modern, real-time, and responsive user interface, a robust backend, and a dedicated background worker for handling long-running tasks.

### Core Features

*   **Image Management**: Syncs and organizes astrophotography images from an [Immich](https://immich.app/) server, a self-hosted photo and video backup solution.
*   **Plate Solving**: Integrates with [Astrometry.net](http://nova.astrometry.net/) to automatically analyze images, identify the stars and celestial objects within them, and retrieve detailed astronomical data.
*   **Dedicated Worker Process**: A separate background worker handles the plate-solving jobs, periodically checking for results from Astrometry.net and updating the application's database without blocking the main server.
*   **Real-time Updates**: The frontend uses WebSockets to provide live status updates for ongoing plate-solving jobs, giving users immediate feedback.
*   **Equipment Management**: Provides functionality to track and manage astrophotography equipment like telescopes and cameras.
*   **Advanced Filtering**: The UI likely allows for filtering and searching images based on various criteria, such as the objects identified, tags, or plate-solving status.

### Technical Architecture

The project is structured as a monorepo-like full-stack application with distinct client, server, and shared codebases.

*   **Frontend**:
    *   **Framework**: React with TypeScript
    *   **Routing**: `wouter`
    *   **Styling**: Tailwind CSS with `shadcn/ui` components (indicated by `components.json` and the UI component files).
    *   **Data Fetching**: `@tanstack/react-query` for managing server state.
    *   **Real-time**: Native browser WebSocket with automatic reconnection and exponential backoff.
    *   **Build Tool**: Vite

*   **Backend**:
    *   **Framework**: Hono with `@hono/node-server` and TypeScript
    *   **Database**: PostgreSQL, managed with the **Drizzle ORM**. The use of `@neondatabase/serverless` suggests it's designed for or deployed on a serverless Postgres platform like Neon.
    *   **Real-time**: Native WebSocket using the `ws` package for communication with the client.
    *   **Background Jobs**: A separate Node.js process (`server/worker.ts`) handles asynchronous tasks, specifically communicating with the Astrometry.net API.

*   **Development & Build**:
    *   **Runtime**: `tsx` is used for running TypeScript code directly in development.
    *   **Build**: `esbuild` is used to build the server and worker for production, while `vite` builds the frontend.
    *   **Task Runner**: `npm scripts` are used to manage development, build, and production startup, with `concurrently` to run multiple processes at once (e.g., server and worker).

In summary, Sidereal is a sophisticated and modern web application that leverages a range of powerful tools to provide a specialized and feature-rich experience for astrophotography enthusiasts.