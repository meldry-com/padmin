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

# Run on port 9090 — the container listens on 8080 by default because
# the image uses the unprivileged nginx variant (nginxinc/nginx-unprivileged).
docker run -p 9090:8080 palpo-admin

# To override the listen port inside the container (e.g. to bind 80 if you
# pair this with a sidecar that needs the lower port), set PADMIN_PORT and
# remember to expose that port:
docker run -p 9090:80 -e PADMIN_PORT=80 palpo-admin
```

The image uses a multi-stage build: Rust/Dioxus compiles the WASM app, then
the unprivileged nginx image serves the static files as a non-root user.
The nginx entrypoint also exposes `/healthz` for container and platform
liveness checks.

GitHub Actions publishes multi-architecture images for `linux/amd64` and `linux/arm64` to GHCR:

```bash
docker pull ghcr.io/meldry-com/padmin:latest
```

## Credential rotation policy

Every secret-bearing field in `examples/pasion.yaml`, `examples/palpo.toml`,
and any compose file shipped here is a **placeholder**. Replace each value
before deploying:

- GitHub / Google / other upstream OAuth apps → register your own and paste
  the `client_id` / `client_secret` (the example file uses
  `REPLACE_WITH_YOUR_GITHUB_CLIENT_SECRET` etc.).
- SMTP credentials → use your own provider; for Gmail generate an App
  Password at <https://myaccount.google.com/apppasswords>.
- Pasion encryption / signing keys → generate fresh material with
  `openssl rand -hex 32` and `openssl genpkey -algorithm RSA …` rather than
  reusing what ships in any sample config.
- Matrix shared secret (`matrix.secret`) → `openssl rand -base64 24`.

If a real secret was ever committed (this repo's history previously
contained one), treat it as compromised and rotate it.

## Full Stack Example

See [`examples/`](examples/) for a complete Docker Compose setup that runs Palpo Admin alongside the Palpo homeserver, Pasion auth service, Element Web client, and PostgreSQL.

```bash
cd examples
docker compose up -d --build
# Open http://localhost:7060
```

The compose example uses the canonical local ports from [`examples/README.md`](examples/README.md):

- `padmin`: `http://localhost:7060`
- `pasion`: `http://localhost:7080`
- `element`: `http://localhost:7070`

## End-to-End Tests

The repository now has a single Playwright workspace rooted at `e2e/`.

```bash
# Main padmin regression suite
npm test

# Example-stack smoke suite against examples/compose.yml
npm run test:example-stack:fresh
```

The example-stack smoke workflow, fixtures, and known limitations are documented in [`e2e/example-stack/README.md`](e2e/example-stack/README.md).

## Project Structure

```
playwright.config.ts              # Main padmin Playwright config
playwright.example-stack.config.ts# Example-stack smoke config
e2e/                             # Unified Playwright workspace
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
  README.md        # Example-stack deployment notes
```

## License

See [LICENSE](LICENSE) for details.
