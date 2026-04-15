# Palpo Admin

A web-based admin dashboard for [Palpo](https://github.com/palpo-im/palpo) Matrix homeserver, built with [Dioxus](https://dioxuslabs.com/) and compiled to WebAssembly.

## Features

- **Dashboard** - Server overview and statistics
- **User Management** - View, search, and manage Matrix users
- **Room Management** - Browse and moderate rooms
- **Media Management** - View and manage uploaded media
- **Server Status** - Monitor server health and configuration
- **Registration Tokens** - Create and manage registration tokens
- **Reports** - Review user/room reports
- **Server Notices** - Send server-wide announcements
- **Auth Status** - View authentication and delegated auth status
- **Destinations** - Manage federation destinations

## Tech Stack

- **[Dioxus](https://dioxuslabs.com/)** - Rust UI framework targeting WebAssembly
- **[gloo](https://gloo-rs.web.app/)** - Rust/WASM utilities (network, storage, timers)
- **[wasm-bindgen](https://rustwasm.github.io/docs/wasm-bindgen/)** - Rust/JavaScript interop
- **Nginx** - Static file server (in Docker)

## Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started): `cargo install dioxus-cli`
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`

## Development

```bash
# Start the dev server with hot reload
dx serve

# Build for production
dx build --release
```

The dev server runs at `http://localhost:8080` by default.

## Docker

```bash
# Build the Docker image
docker build -t palpo-admin .

# Run on port 9090
docker run -p 9090:80 palpo-admin
```

The image uses a multi-stage build: Rust/Dioxus compiles the WASM app, then nginx serves the static files.

GitHub Actions publishes multi-architecture images for `linux/amd64` and `linux/arm64` to GHCR:

```bash
docker pull ghcr.io/meldry-com/padmin:latest
```

## Full Stack Example

See [`examples/`](examples/) for a complete Docker Compose setup that runs Palpo Admin alongside the Palpo homeserver, Pasion auth service, Element Web client, and PostgreSQL.

```bash
cd examples
docker compose up -d --build
# Open http://localhost:9090
```

## Project Structure

```
src/
  main.rs          # App entry point
  router.rs        # Client-side routing
  api/             # HTTP API client for Palpo server
  components/      # Reusable UI components
  pages/           # Page components (dashboard, users, rooms, etc.)
  types/           # Data types and API models
  utils/           # Helper functions
  style.css        # Global styles
examples/
  compose.yml      # Full stack Docker Compose
  palpo.toml       # Palpo homeserver config
  pasion.yaml      # Pasion auth config
  ...
```

## License

See [LICENSE](LICENSE) for details.
