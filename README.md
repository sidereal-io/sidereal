<p align="center">
  <img src="docs/assets/logo.png" width="300">
  <h1 style="font-size: 55px" align="center">SIDEREAL</h1>
</p>

<p align="center">
  <a href="./LICENSE"><img src="https://img.shields.io/badge/License-MIT-green.svg" /></a>
  <a href="https://discord.gg/ffZ8cuJ8Kh"><img src="https://img.shields.io/badge/Discord-Join%20Chat-5865F2?logo=discord&logoColor=white" alt="Join us on Discord" /></a>
</p>

> [!WARNING]
> **Sidereal is being rebuilt on this branch and can't be installed yet.**
> To run Sidereal today, use the [`v0.x` branch](https://github.com/sidereal-io/sidereal/tree/v0.x), which carries the released v0.10.x line and its patches.

**Sidereal** is a self-hosted library for astrophotographers. It manages your images at every stage: calibration frames, raw lights by the hundreds, stacked results, and annotated finals.

Sidereal reads each file's metadata, groups frames into sessions by target, filter and equipment, matches calibration frames to the lights they fit, and records how every stack was made. It never changes your originals. The stacking itself stays in the tools you already use, such as Siril and PixInsight. Every processing step is a plugin, including the built-in ones.

Perfect for organizing, analyzing, and showcasing your astrophotography collection with full control over your data and infrastructure.

Read more about the design in [`docs/architecture.md`](docs/architecture.md).

## Features

Sidereal is being built to provide:

- **Every stage in one library:** calibration frames, raw lights (FITS and XISF), stacks and annotated finals
- **Sessions:** frames grouped by night, target, filter and equipment
- **Calibration matching:** master darks and flats matched to the lights they fit
- **Lineage:** which lights and masters made each stack
- **Safe originals:** never changed, checked by content hash, and repairable
- **Plate solving** with Astrometry.net
- **Equipment and targets:** telescopes, cameras and filters, plus a deep-sky catalog
- **Plugins for every step:** working alongside Siril and PixInsight, importing from and publishing to Immich, Astrobin or a static gallery

## Where to go

| You want to | Go to |
|---|---|
| Install and run Sidereal | The [`v0.x` branch](https://github.com/sidereal-io/sidereal/tree/v0.x) and its README |
| Report a bug in the running app | [Issues](https://github.com/sidereal-io/sidereal/issues). Say which version you run |
| Understand the new design | [`docs/architecture.md`](docs/architecture.md) and [`docs/decisions/`](docs/decisions/) |
| See the build plan | [`openspec/migration.md`](openspec/migration.md) |
| Contribute | [`CONTRIBUTING.md`](CONTRIBUTING.md) |

## Developing on `main`

The server is a Rust cargo workspace in `server/`, and the web interface is a React app in `web/`. The root `justfile` runs both.

### Get set up

Pick one route. [`CONTRIBUTING.md`](CONTRIBUTING.md#-development-environment) has the steps for each.

- **With Nix:** install Nix and direnv, then run `direnv allow` in the repo root. The pinned shell provides every tool.
- **Without Nix:** install rustup, Node 26 (your version manager reads `.nvmrc`), pnpm 12, and [`just`](https://github.com/casey/just).

### Run it

```bash
just dev
```

This starts the server and the web interface together. Open `http://localhost:5173`. Today the page shows the server's health, because the first features are still being built.

### Check it

```bash
just check
```

This is the gate for every pull request. It runs the server checks (format, clippy, tests, and the dependency-direction lint), then the web checks (type check, lint, format check, and unit tests). CI runs the same checks.

### How work is planned

Every change starts as a [GitHub issue](https://github.com/sidereal-io/sidereal/issues). Product changes then go through an [OpenSpec](openspec/) change: a proposal, specs, a design, and tasks, all reviewed in one pull request. Commits follow [Conventional Commits](https://www.conventionalcommits.org/). [`CONTRIBUTING.md`](CONTRIBUTING.md) explains the workflow.

Fixes for the running v0.10.x app go to the `v0.x` branch, not `main`.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Acknowledgments

- **[Immich](https://immich.app/)**: inspiration, and a place to import from and publish to
- **[Astrometry.net](https://astrometry.net/)**: plate solving service and algorithms
- **[Siril](https://siril.org/)** and **[PixInsight](https://pixinsight.com/)**: the stacking tools Sidereal is designed to work alongside

> **Disclaimer**: Sidereal is an independent project. It is not affiliated with, endorsed by, or officially connected to Immich or its developers.

---

<div align="center">

**Built for the astrophotography community**

[Star this repo](https://github.com/sidereal-io/sidereal) | [Report bug](https://github.com/sidereal-io/sidereal/issues) | [Request feature](https://github.com/sidereal-io/sidereal/discussions)

</div>
