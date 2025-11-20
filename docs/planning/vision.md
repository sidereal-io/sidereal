# Sidereal

Sidereal is a self-hosted, privacy-focused photo gallery and asset management system that helps astrophotographers organize, enrich, and manage their images by providing specialized metadata management, intelligent celestial object identification, and equipment tracking.

## Users

### Primary Users
- **Home Astrophotographers:** Hobbyists (from novice to expert) who need a simple, centralized way to organize, manage, and share their images with friends and family.
- **Privacy-Conscious Hobbyists:** Users who prioritize data ownership and security, demanding a self-hosted solution over cloud-based services.
- **Technical Analysts:** Enthusiasts who want to analyze and query their image data using rich, technical metadata (e.g., equipment used, exposure times, object data).
- **Friends and Family:** Viewers who know nothing about the hobby but enjoy viewing and learning details about what the photographer has shared.

### User Personas
**Hobbyist Astrophotographer** (30-60)
- **Role:** Technical Hobbyist
- **Context:** Managing a growing library of personal images captured from a home or remote observatory setup.
- **Pain Points:**
    - General photo tools (Google Photos, Apple Photos) don't support FITS/XISF files or understand astro-specific metadata.
    - Forgetting which equipment (telescope, camera, filter) was used for a specific imaging session.
    - Inability to find photos based on celestial object (e.g., "all my photos of M31") or technical data (e.g., "all images with >1 hour of integration").
    - Manually tracking data in spreadsheets is tedious and error-prone.
- **Goals:**
    - A single, private, and secure platform to see all images.
    - Automatically identify what object is in a photo.
    - Easily compare results from different equipment combinations.
    - Find any image quickly using powerful, relevant search filters.

## The Problem

### Fragmented & Manual Astrophotography Data Management
Astrophotographers capture images in complex scientific formats (FITS, XISF) that general-purpose photo tools cannot parse. This forces users to rely on manual, disconnected systems—like spreadsheets, complex folder names, or memory—to track critical metadata. Information like equipment used, total exposure time, calibration frames, and celestial coordinates is lost or difficult to access.

**Quantifiable Impact:** This fragmentation leads to wasted time, lost data, and an inability to effectively analyze or find images. Novices are overwhelmed by the data management, and experts are forced into inefficient workflows.

**Our Solution:** Sidereal provides a self-hosted "hub" that automatically ingests, catalogs, and enriches astro-images. It uses a system of extensible "Metadata Agents" to analyze images, identify celestial objects, link to equipment inventories, and make all data instantly searchable in a private, secure gallery.

## Differentiators

### Specialized, Extensible Metadata Engine
Unlike general photo managers (like Plex or Photoprism) or simple file browsers, we provide native support for astro-specific formats (FITS, XISF), a pluggable "Metadata Agent" architecture for intelligent data enrichment (like plate solving and object identification), and integrated equipment/object catalogs.

This results in users being able to automatically tag and analyze their work with scientific precision, search by technical parameters (like pixel scale or constellation), and manage their entire hobby from one platform, all while maintaining 100% data privacy.

## Key Features

### Core Features
- **Unified Image Catalog:** Securely store, view, and manage all your astro-images (including FITS, XISF, and RAWs) from one place, with support for both local filesystems and S3-compatible storage.
- **Calibration Frame Management:** Automatically recognize and separate your calibration frames (darks, flats, biases) from your primary "light" images to keep your main gallery clean.

### Enrichment Features
- **Intelligent Metadata Collection:** Automatically enrich your images with deep scientific data using asynchronous, pluggable "Metadata Agents" that run in the background.
- **Equipment Catalog:** Easily track which telescope, camera, and filter were used for every shot by linking images to a register of your personal equipment.

### Advanced Features
- **Comprehensive Metadata & Search:** Instantly find any image using powerful, astro-specific filters like "target object name," "total integration time," "constellation," or "camera model."
- **Celestial Object Catalog:** Discover what's in your photos as the system automatically identifies and tags celestial objects against a managed catalog.