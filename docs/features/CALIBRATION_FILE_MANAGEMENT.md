# Calibration & Raw Photo Organization

## Overview

A file management system that lets users organize, sort, and associate their raw imaging data (lights, darks, flats, bias/offset frames) with specific sessions. Provides structure around the calibration workflow that astrophotographers deal with before stacking.

## Potential Capabilities

### File Organization
- Upload or register raw capture files (FITS, XISF, CR2/ARW, etc.)
- Categorize frames by type: lights, darks, flats, bias/dark flats
- Associate frame sets with specific imaging sessions

### Calibration Library
- Maintain a library of master calibration frames (master dark, master flat, etc.)
- Track calibration frame metadata: temperature, gain, exposure, camera, date
- Match calibration frames to light frames by compatible parameters

### Session Association
- Link raw files/frame groups to sessions from the session tracker
- View a session's complete file inventory (lights + associated calibration frames)
- Track processing status per session (raw, calibrated, stacked, processed)

## Technical Considerations

- File storage: could reference files by path (local/NAS) rather than uploading to Sidereal directly
- Schema additions: tables for frames, calibration library, session-file associations
- Potentially large data volumes — needs to be lightweight and reference-based

## Status

Idea — not yet scoped or designed.
