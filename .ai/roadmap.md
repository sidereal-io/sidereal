# Roadmap

1.  [ ] **Project Setup** - Inititalize a new project following the architecture outlined in `.ai/standards/architecture.md` and the standards outlined in `.ai/standards/*` `S`
1.  [ ] **Equipment Catalog Management** — Implement full-stack CRUD (Create, Read, Update, Delete) for the Equipment Catalog, allowing users to define their telescopes, cameras, and filters. `M`
2.  [ ] **SAL & Local File Ingestion** — Implement the Storage Abstraction Layer (SAL) with a local filesystem provider. Create a backend service to scan specified directories and populate the database catalog with new image records. `L`
3.  [ ] **Basic Image Gallery UI** — Develop a simple frontend view that fetches all images from the catalog API and displays them as a list or thumbnail grid, demonstrating the ingestion process. `S`
4.  [ ] **Metadata Agent Architecture** — Build the core backend system to manage asynchronous, pluggable "Metadata Agents," including database models for tracking job status and errors per image. `L`
5.  [ ] **FITS/XISF Header Agent** — Create the first Metadata Agent to parse FITS/XISF headers, extract key metadata (e.g., exposure, date, target), and save it to the image record in the database. `L`
6.  [ ] **Image-Equipment Linking** — Add UI and API functionality to allow users to manually associate one or more items from their Equipment Catalog with a specific image record. `M`
7.  [ ] **Calibration Frame Management** — Update the image model and UI to allow users to tag images as 'Light', 'Dark', 'Flat', or 'Bias', and provide a basic filter in the gallery to hide/show calibration frames. `S`
8.  [ ] **Advanced Metadata Search** — Implement the backend API and frontend UI filters to allow users to search the gallery based on key metadata, such as Equipment Used, Target Object Name, or Integration Time. `M`

> Notes
> - Order items by technical dependencies and architecture
> - Each item should represent an end-to-end (frontend + backend) functional and testable feature