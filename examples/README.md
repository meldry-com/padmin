# Palpo Stack Example

A complete local development stack running a Matrix homeserver with OAuth/OIDC authentication, an admin dashboard, and Element Web client.

## Services

| Service | Description | Host Port | Internal Port |
|---------|-------------|-----------|---------------|
| **postgres** | PostgreSQL database (shared by palpo and pasion) | `15432` | `5432` |
| **palpo** | Matrix homeserver (Client-Server API) | `8008`, `8448` | `8008`, `8448` |
| **pasion** | OAuth 2.0 / OpenID Connect authentication service | `7080` | `7080` |
| **padmin** | Admin dashboard web UI (nginx + Dioxus WASM) | `7060` | `80` |
| **element** | Element Web Matrix client | `7070` | `80` |

## Quick Start

```bash
# 1. Review and adjust configuration files as needed
#    - palpo.toml    (Matrix homeserver config)
#    - pasion.yaml   (OAuth/OIDC config)
#    - element-config.json (Element Web config)

# 2. Build and start all services
docker compose up -d --build

# 3. Open the services in your browser
#    Admin dashboard:  http://localhost:7060
#    Auth service:     http://localhost:7080
#    Element client:   http://localhost:7070
#    Matrix API:       http://localhost:8008
```

## Smoke Tests

The example-stack smoke tests now live in the root Playwright workspace rather
than under `examples/e2e/`.

Run them from the repository root:

```bash
npm install
npx playwright install chromium
npm run test:example-stack:fresh
```

If you need to inspect the reset flow separately:

```bash
npm run stack:reset
npm run test:example-stack
```

The smoke-suite implementation and caveats are documented in
[`../e2e/example-stack/README.md`](../e2e/example-stack/README.md).

## Architecture

```
Browser
  |
  +---> :7070  Element Web  ----+
  +---> :7080  Pasion (Auth) ---+--> :8008 Palpo (Matrix) --> :5432 PostgreSQL
  +---> :7060  Padmin (Admin) --+                                    |
                                                                     |
         Pasion (Auth) --------------------------------------------->+
```

- **Palpo** is the Matrix homeserver handling Client-Server API and federation.
- **Pasion** provides OAuth 2.0 / OIDC authentication. Palpo delegates auth to Pasion via `delegated_auth` in `palpo.toml`.
- **Padmin** is a static Dioxus WASM app served by nginx for managing the homeserver.
- **Element** is the standard Matrix web client, preconfigured to connect to the local Palpo instance.
- **PostgreSQL** hosts two databases: `palpo` (homeserver data) and `pasion` (auth data), initialized by `init-db.sh`.

## Configuration Files

| File | Purpose |
|------|---------|
| `palpo.toml` | Palpo homeserver settings: server name, database, federation, delegated auth |
| `pasion.yaml` | Pasion OAuth/OIDC: database, Matrix integration, password policy, upstream providers, email |
| `pasion-signing-key.pem` | Signing key for Pasion JWT tokens |
| `element-config.json` | Element Web client config: homeserver URL and server name |
| `init-db.sh` | PostgreSQL entrypoint script that creates the `pasion` database |
| `nginx.conf` | Reference nginx config for serving Dioxus WASM apps (not mounted by default) |

## Database

- **User**: `palpo`
- **Password**: `changeme`
- **Host**: `localhost:15432` (from host) or `postgres:5432` (from containers)
- **Databases**: `palpo`, `pasion`

Connect from host:

```bash
psql -h localhost -p 15432 -U palpo -d palpo
psql -h localhost -p 15432 -U palpo -d pasion
```

## Default Credentials and Secrets

> **Warning**: These are development defaults. Change them before any non-local deployment.

| Setting | Value | File |
|---------|-------|------|
| Postgres password | `changeme` | `compose.yml`, `palpo.toml`, `pasion.yaml` |
| MAS shared secret | `replace-with-a-random-secret` | `palpo.toml`, `pasion.yaml` |
| Encryption key | `0a1b2c...` (hex string) | `pasion.yaml` |

## Upstream OAuth Providers

Pasion is preconfigured with a GitHub OAuth provider. To use it:

1. Create a GitHub OAuth App at <https://github.com/settings/developers>.
2. Set the **Authorization callback URL** to:
   ```
   http://localhost:7080/upstream/callback/<provider_id>
   ```
   where `<provider_id>` matches the `id` field of the provider in `pasion.yaml` (e.g. `01KMQDVNFWTRF9K8FV8FCFKARM`).
3. Update `client_id` and `client_secret` in `pasion.yaml` with your app's credentials.

> **Note**: The `redirect_uris` for the `Palpo Admin Dashboard` client in `pasion.yaml` must match the host port padmin is served on (`http://localhost:7060/oauth/callback`), not the Pasion port.

## Volumes

| Volume | Purpose |
|--------|---------|
| `postgres_data` | PostgreSQL data directory |
| `palpo_media` | Uploaded media files |

To reset all data:

```bash
docker compose down -v
```

## Useful Commands

```bash
# View logs for a specific service
docker compose logs -f pasion

# Rebuild a single service
docker compose build padmin

# Restart a single service
docker compose restart palpo

# Stop everything
docker compose down

# Stop and remove all data
docker compose down -v
```
