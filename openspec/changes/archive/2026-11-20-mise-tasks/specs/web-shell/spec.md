<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## MODIFIED Requirements

### Requirement: The web shell's dev server stays on this machine

The web shell's dev server SHALL listen on port 5173 and on loopback addresses only. When port 5173 is taken, the dev server SHALL stop with an error instead of listening on another port.

#### Scenario: A contributor starts the web shell

- **WHEN** a contributor starts the web shell with port 5173 free
- **AND** a tester lists listening TCP sockets with `ss -ltnp`
- **THEN** the web shell's dev server listens on port 5173
- **AND** every address it listens on is `127.0.0.1` or `[::1]`

#### Scenario: Port 5173 is taken

- **WHEN** another process listens on port 5173, and a contributor runs `mise run web`
- **THEN** the command exits with a non-zero status
- **AND** the web shell's dev server listens on no port
