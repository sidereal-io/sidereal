# 🌌 Sidereal: Self-Hosted Astrophotography Management System

## 1. 🌟 Introduction

**Sidereal** is a **self-hosted, privacy-focused photo gallery and asset management system** designed specifically for astrophotographers. It addresses the critical need for specialized metadata management, intelligent celestial object identification, and equipment tracking—all missing in general-purpose photo management tools.

The primary goal is to provide a single, secure platform for astrophotographers to store their images and automatically enrich them with the highly technical, scientific metadata required for the hobby, without sacrificing data ownership or privacy.

The Sidereal project will be considered a success if it achieves the following:

* **Completion Rate:** Plate Solve Agent successfully solves >90% of suitable deep-sky images.
* **Performance:** Image metadata is fully enriched (all agents run) within 5 minutes of upload (assuming a modest self-hosted server).
* **Extensibility:** Successful integration of a user-contributed, third-party Metadata Agent.

## 2. 🎯 Core Features and Capabilities

Sidereal must deliver the following core capabilities, distinguishing it from general photo management software:

### 2.1. Specialized Image Storage

* **Format Support:** Native support for display and analysis of **FITS**, **XISF**, TIFF, and common RAW/JPEG formats.
* **Storage Flexibility:** Support for accessing files via a **Storage Abstraction Layer (SAL)**, allowing the user to choose between **Local Filesystem** or **S3-Compatible Storage** (e.g., MinIO, AWS S3).
* **Calibration Frame Handling:** Ability to recognize and hide calibration frames (Darks, Flats, Biases) from the main gallery view while maintaining links to them for audit purposes.

### 2.2. Intelligent Metadata Workflow (Metadata Agents)

* The system uses **pluggable, user-managed Metadata Agents** to enrich image data asynchronously.
* **Workflow Triggers:** Agents can be triggered upon **new file detection**, **user update**, or **manual UI action**.
* **Agent Types (Initial Set):**
  * **File Agent:** Reads basic EXIF/FITS data, calculates checksums, and records file information.
  * **Plate Solve Agent:** Determines celestial coordinates (RA/Dec), Field of View, and Pixel Scale.
  * **Integration Agent:** Calculates the **Total Integration Time by Filter** (e.g., Ha, OIII, L).
  * **Object Catalog Agent:** Identifies all **Deep-Sky Objects (DSOs)** (Messier, NGC, IC) within the Plate Solve coordinates.
  * **Description Agent:** Fetches and applies descriptive text for the primary object (e.g., pulling a description from **Wikipedia**).

### 2.3. Comprehensive Metadata & Search

* **Searchable Metadata:** Advanced filtering based on: **Total Integration Time** (range), **Specific Equipment** (e.g., Camera Model), **Target Object Name**, **Celestial Hemisphere**, and **Constellation**.
* **Location Metadata:** Includes **Celestial Hemisphere**, **Constellation**, **Center Coordinates (RA/Dec)**, **Field Size**, and **Pixel Scale**.
* **Time Tracking:** Records **Date Captured**, **Date Created**, and **Date Modified**.

### 2.4. Equipment & Object Catalogs

* **Equipment Catalog:** A register of user-owned equipment, including types: **Telescope**, **Camera**, **Mount**, **Filter**, **Accessory**, and **Software**. Images are linked to the specific equipment used.
* **Celestial Object Catalog:** A managed list of DSOs used by the Object Catalog Agent for identification and tagging.

### 2.5. Image Catalog

The Sidereal Image Catalog is a centralized system that keeps track of metadadta for all of images in your collection.  The catalog is built around the concept of _______ that contribute metadata about each image which that then be visualized in the UI.

The catalog forms a hub of sorts, where images are ingested from various sources.  A record of each image is held in a database, subject to automated processing, and then presented through an API for quick and easy access by the UI. The most common source of images is the file system. The catalog makes sure to keep itself up to date with changes to those files.

The main extension points where developers can customize the catalog are:
- Image **Providers**, that feed initial raw images into the catalog,
- Metadata **Agents**, that analyze and contribute metadata for each image in the catalog.

### 2.6 The Lifecycle of an Image

The high level processes involved are:
- Ingestion, where image providers fetch raw images from external sources and register an Image Record in the database,
- Processing, where agents continually treat the ingested images and may emit metadata or errors about it,
- Stitching, where all of the metadata emitted by various agents are assembled together into the final output.

An image is not visible to the outside world (through the catalog API), until it has passed through the last process and landed among the final images.

#### 2.6.1. Ingestion

Image Providres are responsible for fetching images from external sources in any way that they see fit, translating those into image records, and notifying the database when those images are added or removed. These are the unprocessed images that will be subject to later processing (see below), and they form the very basis of existence for an image. If there were no image providers, no images would ever enter the system.

The database always keeps track of the set of images that belong to each provider; no two providers can try to output the same image. And when a provider signals the removal of an image, then that leads to an eager deletion: the image and all auxiliary data that it has led to in the database is immediately purged.

#### 2.6.1. Processing

Every unprocessed image comes with a timestamp, which tells at what time that the processing loop should next try to process it. When the image first appears, this timestamp is set to "now" - asking for it to be picked up as soon as possible.

Processors are responsible for receiving unprocessed image that the server decided are due for processing, and then running that image  through a number of processing stages, analyzing the image and emitting additional metadata about it. When all of that is done, the server takes all of that information and stores it as the processed image record, metadata, and errors separately.

Images are always processed one by one, but all of your catalog service hosts collaborate in doing so to distribute the load. Note how each processor can contribute to one or more of the fixed steps in the processing pipeline. First all of the processors' contributions to one step are run in the order that the processors were registered, then all of their contributions to the next step in the same order, and so on.

#### 2.6.3. Stitching

## 3. ⚙️ Design Decisions & Architectural Choices

### 3.1. Design Philosophy
* **Self-Hosted Focus:** Prioritize simplicity of installation and operation via containerization (**Docker**).
* **Extensibility:** The **Metadata Agent** pattern ensures the system can easily support future astrometry tools, new processing metrics, and user-developed plugins.
* **AWS Portability:** Architectural choices ensure minimal refactoring if the user decides to migrate the management layer to AWS managed services (e.g., using RDS).

### 3.2. Technical Stack and Rationale

| Component | Technology | Rationale |
| :--- | :--- | :--- |
| **Backend / API** | **Node.js** | Unified language stack with the frontend, excellent I/O performance for file handling, and strong async capabilities for job orchestration. |
| **Frontend / UI** | **Node.js (React/Vue)** | Provides a fast, modern, and responsive user interface consistent with the backend language. |
| **Metadata Agents** | **Node.js** | Unified language stack with the frontend,excellent I/O performance for file handling, and strong async capabilities for job orchestration. |
| **Database** | **PostgreSQL** | Provides **ACID compliance** and robust **SQL querying** critical for complex metadata filtering and relational integrity. Highly portable to **AWS RDS/Aurora**. |
| **Deployment** | **Docker/Docker Compose** | Simplifies deployment, ensuring all components run correctly and are easily managed. |


## 4. 🛡️ Non-Functional Requirements (NFRs)

These requirements define the criteria that will be used to judge the operation of the system, rather than specific behaviors.

### 4.1. Security and Privacy 🔒

Given the **self-hosted, privacy-focused** nature of the project, security and data control are paramount.

* **Authentication:** The system must implement robust, role-based authentication with support for at least a **username/password credential store**. Support for **multi-factor authentication (MFA)** is a high-priority future enhancement.
* **Authorization:** Access to administrative functions (e.g., managing agents, system configuration) must be restricted to authenticated **administrator-level** users.
* **Data-in-Transit:** All client-server communication (UI and API) must be secured using **HTTPS/TLS**. The deployment guide must strongly recommend the use of a reverse proxy (e.g., Nginx, Caddy) for certificate management.
* **Configuration Security:** Sensitive configuration data (e.g., database credentials, API keys) must be managed via **environment variables** and never hardcoded.

### 4.2. Performance and Scalability

These requirements address the speed and capacity of the system, especially under typical self-hosted load.

* **User Interface (UI) Responsiveness:** The main gallery view must load and render a page of **50 image thumbnails** and associated summary metadata in **under 2 seconds** on a modern browser.
* **Metadata Search Performance:** Complex metadata queries (e.g., filtering by three different criteria like target, integration time range, and equipment) must return results in **under 3 seconds**.
* **Job Processing Throughput:** The system must be capable of processing the Metadata Agent chain for **10 concurrent image uploads** without significant degradation in job completion time.
* **Storage Scalability:** The architecture must efficiently handle a library size of up to **10,000 processed images** and their associated calibration frames.

### 4.3. Maintainability and Extensibility

These requirements ensure the system is easy to evolve and operate.

* **API Documentation:** The **Node.js** must be documented using a standard like **OpenAPI/Swagger** to facilitate user integration and future development.
* **Logging:** The system must employ structured logging (e.g., JSON logs) with defined severity levels to simplify debugging across the different services (**Node.js, PostgreSQL, Docker**).
* **Configuration Management:** All system configuration should be consolidated into a minimal number of configuration files/environmental variables, following the **12-Factor App** principles where applicable.
* **Documentation:** Comprehensive documentation for **installation (via Docker Compose)**, **configuration (Storage Abstraction Layer setup)**, and **Metadata Agent creation/management** must be provided.

### 4.4. Operational Requirements

These focus on the ease of deployment and ongoing operation in a self-hosted environment.

* **Backups:** Clear instructions and tools/scripts must be provided for backing up the **PostgreSQL Database**.
* **Monitoring:** Basic system health endpoints (e.g., `/health` or `/status`) must be available on the Node.js API to indicate the status of connected services (API, PostgreSQL, Agents).
* **Installation Simplicity:** The primary deployment method using **Docker Compose** must require only running a single command (`docker compose up`) after configuring environment variables.
