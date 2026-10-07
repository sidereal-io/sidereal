<p align="center">
  <img src="https://raw.githubusercontent.com/sidereal-io/sidereal/v0.x/apps/client/public/logo.png" width="300">
  <h1 style="font-size: 55px" align="center">SIDEREAL</h1>
</p>

<p align="center">
  <a href="./LICENSE"><img src="https://img.shields.io/badge/License-MIT-green.svg" /></a>
  <a href="https://discord.gg/ffZ8cuJ8Kh"><img src="https://img.shields.io/badge/Discord-Join%20Chat-5865F2?logo=discord&logoColor=white" alt="Join us on Discord" /></a>
</p>

> [!WARNING]
> **Sidereal is being rebuilt on this branch and can't be installed yet.**
> To run Sidereal today, use the [`v0.x` branch](https://github.com/sidereal-io/sidereal/tree/v0.x), which carries the released v0.10.x line and its patches.

**Sidereal** is a self-hosted photo gallery and management system for astrophotographers. It provides plate solving with [Astrometry.net](https://astrometry.net/), equipment tracking, and metadata for deep-sky imaging.

This branch, `main`, holds the next version: a Rust server and a React web interface, built around a plugin-based imaging pipeline. The first release from `main` is planned as `v2.0.0`. Until then, everything here is under active development.

## Where to go

| You want to | Go to |
|---|---|
| Install and run Sidereal | The [`v0.x` branch](https://github.com/sidereal-io/sidereal/tree/v0.x) and its README |
| Report a bug in the running app | [Issues](https://github.com/sidereal-io/sidereal/issues). Say which version you run |
| Understand the new design | [`docs/architecture.md`](docs/architecture.md) and [`docs/decisions/`](docs/decisions/) |
| See the build plan | [`openspec/migration.md`](openspec/migration.md) |
| Contribute | [`CONTRIBUTING.md`](CONTRIBUTING.md) |

## Developing on `main`

Clone the repository, then run the server and the web interface together:

```bash
just dev
```

Before opening a pull request, run the checks:

```bash
just check
```

[`CONTRIBUTING.md`](CONTRIBUTING.md) explains the tools you need, with or without Nix. [`server/README.md`](server/README.md) and [`web/README.md`](web/README.md) cover each part.

Fixes for the running v0.10.x app go to the `v0.x` branch, not `main`.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Acknowledgments

- **[Immich](https://immich.app/)**: inspiration and integration for photo management
- **[Astrometry.net](https://astrometry.net/)**: plate solving service and algorithms

> **Disclaimer**: Sidereal is an independent project. It is not affiliated with, endorsed by, or officially connected to Immich or its developers.
