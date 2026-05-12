# N.I.N.A. Plugin Integration

## Overview

A plugin for N.I.N.A. (Nighttime Imaging 'N' Astronomy) that integrates with Sidereal's session tracker. Allows imaging session data to flow directly from the capture software into Sidereal without manual entry.

## Potential Capabilities

- Automatically push session metadata (start/end time, target, equipment, conditions) to Sidereal as a session is captured
- Sync frame/sub-exposure details (filter, exposure time, gain, temperature, etc.)
- Report session progress in real-time or on completion
- Leverage NINA's sequencer events to trigger data pushes

## Technical Considerations

- NINA plugins are written in C# / .NET
- Would need a Sidereal API endpoint to receive session data (e.g., `POST /api/sessions/import`)
- Authentication mechanism for the plugin to talk to the server
- Could be a separate repo or a `plugins/` directory in this project

## Status

Idea — not yet scoped or designed.
