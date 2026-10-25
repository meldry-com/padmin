> Historical reference migration record from 2026-10-23. Current repository/branch integration and verification: [glory-branch-integration.md](glory-branch-integration.md).

# Build, deployment and original behavior acceptance

2026-10-23. The evidence below is from the final release artifact with WASM SHA-256
`258938a5bb72b5f13a344fdfa92c1df53303cfddaf5217d0dacf9752c3e5f3f1` and application
source digest `57cb486530aa9aecdbab8fffb2ebe599775fee897a83a93347560abbd76b3e8c`.

## Actual serving and authentication

`D:/BuildCache/padmin-glory/acceptance-deployment.json` records four successful
commands and their evidence paths:

- `node scripts/native-nginx-acceptance.cjs`: root, `/admin` and `/console/manage`
  serve the actual release JS/WASM directly from D:, then complete real Pasion
  password login, consent, PKCE, Matrix whoami and both administrator probes.
  Actual user collection, direct refresh, fresh OAuth after refresh, room navigation,
  query-preserving 308 and strict missing-WASM 404 also pass.
- `node scripts/native-nginx-acceptance.cjs --same-origin`: repeats those three
  mounts with Pasion at same-origin `/auth`. Pasion JS, WASM and CSS return 200;
  authorization, login, consent, token exchange and administrator API retain `/auth`.
- `node scripts/nginx-acceptance.cjs`: the production entrypoint runs as the
  `nginx` user in an already cached nginx container. Its health, prefix redirect,
  history fallback, runtime config, strict static 404, CSP/security headers and
  actual Matrix proxy pass at all three mounts. This container contains a source
  index only, so this command alone is not evidence of full application rendering.
- `python -B scripts/config-serving-acceptance.py`: a clean D: output directory
  gets valid `config.json`, including correct JSON encoding for quotes, backslashes
  and Unicode. Actual HTTP `/admin/config.json` and the release WASM login view
  pass without fixtures. The static development server performs no API proxying.

The full native nginx checks use the configuration emitted by the production
entrypoint, adapting only filesystem/MIME/PID/temp paths, the local upstream host
and test port. `native-site/pkg` is a D: junction to the release output. No compiled
JS/WASM was copied into Docker's C-backed storage. No image build or pull was run.
Docker Desktop's existing builder storage is on C:, so image creation remains an
operator prerequisite requiring a D-backed or remote builder. Native nginx 1.31.2
was extracted under the D: task tools directory from the official Windows ZIP.

Real backend prerequisites were made usable rather than replaced by fixtures:
isolated native PostgreSQL data is under `real-stack/postgres` on D:, Pasion uses
an existing binary with fresh task-only keys/config, and cached Palpo executes its
existing binary against those isolated databases. An internal Caddy relay makes
the exact public Pasion issuer reachable from Palpo's isolated container. The
real administrator is provisioned through Pasion's actual CLI and confirmed by
server probes; the application never accepts a storage flag as authority.

## Serving corrections verified by these tests

- Serve the current `/pkg` output, preserve security headers on static errors,
  avoid year-long immutable caching for unhashed filenames, and return relative
  mount redirects so a container listen port cannot leak into the public URL.
- Strip the explicitly configured auth proxy prefix and give proxy routes priority
  over the static-file regex. Otherwise token requests and auth assets returned 404.
- Correct confirmed absolute JS-to-WASM paths inside the auth proxy. Use an external
  `pasion-config.js` bridge to decode the upstream public configuration as inert
  JSON, retaining CSP without `unsafe-inline` or evaluation of arbitrary text.
- Use a Pasion frontend build that supports runtime router prefixes. The source
  project's old `dist` was from September; an existing October 15 build was copied
  into the D: acceptance assets directory. The original checkout was not modified.
- `glory.py build/serve` always writes public config. URL/client/sidecar parameters
  use explicit flags or documented environment variables and JSON encoding.

## Original suites

`e2e-behavior-map.json` preserves all 18 imported suites, 77 scenarios and 307
assertion lines. Each reviewed scenario points to a current executable check,
specific result case and a description of DOM or protocol differences. The
redesigned finite records and accessible controls replace old framework-specific
table/class selectors; the old test files are not presented as having executed
unchanged. Current `npm test` and `npm run test:real` execute the replacement suites
and reject missing/stale evidence or artifact hashes.

There are no unmapped scenarios. Seventy-four have executed semantic-equivalent
checks. The three explicit exceptions are:

1. The old Policy Data scenario names a route absent from the original router,
   original Pasion page tree and authorized 33-route plan. It remains recorded as
   an obsolete source assertion, rather than an invented successful page.
2. Current Palpo legitimately advertises application-service and token flows with
   configured SSO. Acceptance requires actual SSO and absence of local password
   exchange; it does not assert the obsolete singleton SSO array.
3. Current Pasion rejects an HTTP loopback redirect with an explicit port. The
   real checks assert that exact 400 and successful 201 registration of a valid
   HTTPS client, including actual ID and redirect URI.

The Appservice namespace scenario is implemented through an application precheck
against actual registrations because the backend itself accepts duplicate claims.
The check rejects exact same-class regex claims when either is exclusive, fails
closed if a registration read fails, preserves the first registration and emits no
second POST. It does not promise general regex-overlap detection or atomic rejection
of another client's simultaneous registration.

The optional external operations sidecar was not available in local checkouts or
running services. `palpo-operations.exe` is an unrelated mini-app coordinator,
not that HTTP API. Its UI, typed request bodies, confirmations, commands and
one-time/recurring schedule lifecycle are covered with stateful protocol fixtures;
no actual sidecar execution is claimed. Unsupported real Palpo operations, including
experimental features and quarantine/protect in this backend, retain visible typed
errors. They never become fabricated success responses.

## Reference cleanup clearance

The build/deployment/authentication/Matrix integration and original-scenario mapping
workstream has no remaining migration implementation item requiring old UI source.
The main final fixture runner also reports exit code 0. All 69 files in
`migration/source-ui` have now been removed; `docs/reference-removal.json` records
each path and hash. Historical test assertions and the baseline hash inventory
remain independent evidence after that cleanup.
