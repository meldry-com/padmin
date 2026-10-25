> Cargo sources: Glory and Sapid use their GitHub `.git` URLs and `main` branches. `Cargo.lock` fixes the resolved commits; build commands use that same Git dependency graph with D: caches. See [dependency source handling](docs/dependency-sources.md).

> Visual restoration (2026-10-25): layouts were checked against the actual original app and rebuilt with official Glory/Sapid main. See the [visual verification record](docs/visual-migration-2026-10-25.md) for versions, coverage and reproducible commands.

> The current UI uses Glory and Sapid. The [branch integration record](docs/glory-branch-integration.md) and October 23 reports describe earlier stages. Local build/test output stays on D:; Docker builds require verified D: or remote builder storage.

# Palpo Admin

A web-based admin dashboard for [Palpo](https://github.com/palpo-im/palpo) Matrix homeserver, built with Glory and compiled to WebAssembly.

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

- **Glory** - Rust UI framework targeting WebAssembly
- **Sapid** - Shared semantic action buttons, with the existing application theme
- **[gloo](https://gloo-rs.web.app/)** - Rust/WASM utilities (network, storage, timers)
- **[wasm-bindgen](https://rustwasm.github.io/docs/wasm-bindgen/)** - Rust/JavaScript interop
- **Nginx** - Static file server (in Docker)

## Prerequisites

- Python 3.11+
- [Rust](https://rustup.rs/) (latest stable)
- wasm-bindgen CLI 0.2.129 (install under `D:/BuildCache/meldry-visual-20261025/padmin/tools`)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`

## Development

```bash
# Start the local development server
python scripts/glory.py serve --dev

# Build for production
python scripts/glory.py build
```

The dev server runs at `http://localhost:8080` by default.

## Docker

```bash
# Build the Docker image
powershell -File scripts/package-container.ps1
# After verifying D: or remote builder storage:
docker build -t palpo-admin D:/BuildCache/meldry-visual-20261025/padmin/container

# Run on port 9090 — the container listens on 8080 by default because
# the image uses the unprivileged nginx variant (nginxinc/nginx-unprivileged).
docker run -p 9090:8080 -e PALPO_URL=http://your-palpo:8008 -e PASION_URL=http://your-pasion:7080 palpo-admin

# To override the listen port inside the container (e.g. to bind 80 if you
# pair this with a sidecar that needs the lower port), set PADMIN_PORT and
# remember to expose that port:
docker run -p 9090:8081 -e PADMIN_PORT=8081 -e PALPO_URL=http://your-palpo:8008 palpo-admin
```

The image serves prebuilt Glory JS/WASM assets through unprivileged nginx.
The packaging script stages the output on D:. Verify Docker builder storage
separately before building the runtime image.
The nginx entrypoint also exposes `/healthz` for container and platform
liveness checks.

CI resolves Glory and Sapid Git sources at the commits in Cargo.lock, builds and tests the WASM on remote
Linux runners, then smoke-tests and publishes the existing multi-platform image
to GHCR and optional Docker Hub on the original push/tag triggers.

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
npm run test:source-baseline:example-stack:fresh
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

## Hosting under `/admin`

Set `PADMIN_BASE_PATH=/admin`, `PADMIN_PASION_BASE_PATH=/auth` and
`PASION_PUBLIC_URL=https://YOUR_WORKSPACE/auth`.
The edge proxy must preserve `/admin/` when forwarding to Padmin (port 8080
by default), and preserve `/auth/` when forwarding to Pasion. Configure the
Pasion listener with `prefix: /auth`, set `http.public_base` and `http.issuer`
to `https://YOUR_WORKSPACE/auth/`, and register the Padmin callback as
`https://YOUR_WORKSPACE/admin/oauth/callback`.

Matrix routes (`/_matrix/`, `/_palpo/`) stay on the workspace origin and point
to Palpo. Padmin loads its runtime configuration, JS, WASM and static assets
under `/admin/`. Empty `PADMIN_BASE_PATH` and `PADMIN_PASION_BASE_PATH` retain standalone
root hosting and its original proxy endpoints. Both mount variables support
other paths, including nested paths such as `/console/manage` and `/identity`.
Match the Pasion listener prefix, public URL, issuer and OAuth callback to those
configured paths; the edge proxy must preserve the prefixes.

Override `PADMIN_MOUNT_PATH` and `PASION_MOUNT_PATH` in the tests to cover custom
mounts, or set both to empty strings to cover standalone root hosting.

Verify a running isolated stack with
`PADMIN_MOUNT_TEST_URL=http://127.0.0.1:17060 npx playwright test -c playwright.mount.config.ts`.

## Acceptance

Set `WASM_BINDGEN=D:/BuildCache/meldry-visual-20261025/padmin/tools/bin/wasm-bindgen.exe`, then build with `python scripts/glory.py build --dev` (omit `--dev` for release). Install the browser driver on D: with `npm install --prefix D:/BuildCache/meldry-visual-20261025/padmin/runtime --cache D:/BuildCache/meldry-visual-20261025/padmin/npm-cache --ignore-scripts playwright@1.63.0`. Tests use installed Microsoft Edge and keep browser profiles on D:.

- `npm test`: actual WASM protocol fixtures, authentication races, keyboard/IME/accessibility, responsive navigation and all domain write suites. It starts a static host if port 8789 is free. Every mutation must have a registered typed fixture.
- `powershell -File scripts/start-real-stack.ps1`: optional isolated real integration prerequisites using existing Pasion/PostgreSQL binaries and an already cached Palpo image. It performs no image build or pull and never changes source checkouts.
- `node scripts/real-server.cjs`, then `npm run test:real`: actual OAuth password/consent/PKCE, Matrix/Pasion administration, registration/email verification, two-user messaging and report persistence. Override `PADMIN_REAL_USERNAME`/`PADMIN_REAL_PASSWORD` for another isolated account.
- `node scripts/nginx-acceptance.cjs`: production entrypoint/non-root nginx contracts against the prepared cached runtime container. `node scripts/native-nginx-acceptance.cjs --same-origin` additionally serves the actual D: JS/WASM through native nginx with root, `/admin` and `/console/manage` app mounts and `/auth` Pasion. This needs the prepared isolated integration services and official nginx 1.31.2 extracted under `D:/BuildCache/meldry-visual-20261025/padmin/tools/nginx`.

The runners reject stale source/build hashes, missing WASM, mismatched served artifacts, missing evidence and nonzero suites. `D:/BuildCache/meldry-visual-20261025/padmin/acceptance-results.json` binds suite exit codes and evidence paths to the actual WASM SHA-256. `test:source-baseline` retains the imported 18 suites as historical assertions; their old framework-specific selectors and storage-token injection are not the current test entry point. The mapping documents replacement checks and three explicit baseline/backend exceptions.

The development server in `glory.py serve` serves static output. `real-server.cjs` forwards same-origin APIs to configured `PALPO_URL`/`PASION_URL`. The nginx deployment supports configured app/auth prefixes while keeping CSP free of `unsafe-inline`; the small external Pasion configuration bridge parses public JSON without evaluation.

`glory.py build/serve` always generates `config.json`. Set `--pasion-public-url` (or `PASION_PUBLIC_URL`), `--oauth-client-id` (or `PADMIN_OAUTH_CLIENT_ID`) and `--palpo-admin-url` (or `PALPO_ADMIN_URL`) as needed. Empty URLs use same-origin defaults and the client defaults to the example registration. This static server has no backend proxy; configure a reverse proxy or use `real-server.cjs` for a real session.
