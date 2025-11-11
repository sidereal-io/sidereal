# Architecture

This document outlines the technical stack choices for the Sidereal project.

## Tech Stack

| Component | Technology | Rationale |
| :--- | :--- | :--- |
| **Backend / API** | **Node.js** | Unified language stack with the frontend, excellent I/O performance for file handling, and strong async capabilities for job orchestration. |
| **Frontend / UI** | **Node.js (React/Vue)** | Provides a fast, modern, and responsive user interface consistent with the backend language. |
| **Database** | **PostgreSQL** | Provides **ACID compliance** and robust **SQL querying** critical for complex metadata filtering and relational integrity. Highly portable to **AWS RDS/Aurora**. |
| **Deployment** | **Docker/Docker Compose** | Simplifies deployment, ensuring all components run correctly and are easily managed. |