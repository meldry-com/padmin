# padmin end-to-end test suite

Playwright smoke suite that drives the `examples/compose.yml` stack
(`palpo` + `pasion` + `padmin` + `element`) through a multi-user scenario:
register three users, log one into padmin as admin, create an appservice,
bring Element up as alice, create a room, register additional users, and
assert that padmin reflects the final state.

## Layout

```
examples/e2e/
├── fixtures/
│   ├── compose.override.yml   # mounts pasion.test.yaml + test env
│   └── pasion.test.yaml       # pasion config with blackhole email
├── helpers/
│   ├── db.ts                  # postgres query + flag patchers
│   ├── element.ts             # Element UI helpers
│   ├── matrix-admin.ts        # palpo mas admin API wrapper
│   ├── padmin.ts              # padmin Dioxus helpers
│   └── pasion.ts              # pasion account/register/login helpers
├── scripts/
│   └── reset-stack.mjs        # down -v, up, crash-loop workaround, health wait
├── tests/
│   └── full-flow.spec.ts      # the suite
├── package.json
├── playwright.config.ts
└── tsconfig.json
```

## Running

```bash
cd examples/e2e

# one-time setup
npm install
npx playwright install chromium

# bring up a clean stack (wipes volumes)
npm run reset

# run the suite
npm test

# debug helpers
npm run test:headed           # headed browser
npm run test:debug            # Playwright Inspector
npm run report                # last HTML report
```

`reset-stack.mjs` is a thin wrapper over
`docker compose -f ../compose.yml -f fixtures/compose.override.yml`. The
override file swaps the committed `pasion.yaml` for a blackhole-email copy
because the example stack otherwise tries to reach Gmail SMTP and blocks
the pasion notification queue worker on the TLS handshake.

## Known pasion-side issues this suite works around

The tests treat the `debug`-build pasion container in the example as the
system under test, so the workarounds are in the *test runner* — we don't
patch pasion itself. Keep an eye on these; if upstream fixes them, the
workarounds should be removed.

1. **Gmail SMTP blocks the notification worker**
   The committed `pasion.yaml` points `email.transport` at Gmail with a
   stale app password. Every verification-email send hangs on the TLS
   handshake and eventually poisons the job queue. We mount
   `fixtures/pasion.test.yaml` instead, which uses
   `email.transport: blackhole`. The verification codes still land in
   `user_email_authentication_codes`; the `getLatestEmailCode` helper
   reads them straight out of postgres.

2. **`queue_leader_queue_worker_id_fkey` cold-boot crash loop**
   Pasion's queue leader election races worker registration on cold
   boot: the leader lease INSERT references a `queue_workers.id` that
   hasn't been committed yet, so postgres raises a FK violation and the
   whole process exits. Docker's `restart: unless-stopped` then puts
   pasion into a crash loop. `reset-stack.mjs` stops pasion, drops the
   FK constraint, truncates `queue_leader` + `queue_workers`, then
   starts pasion back up. The trade-off is a potential dangling leader
   row if a worker dies mid-tick — acceptable for a test stack, not for
   production.

3. **`mark_as_completed` race under load**
   During high concurrency pasion sometimes shuts down with
   `Expected 1 rows to be affected, but 0 rows were affected` when a
   `queue_jobs` update finds the row already moved on. We haven't found
   a clean workaround; the Playwright helpers retry navigations that
   die with `ERR_EMPTY_RESPONSE` / `ERR_CONNECTION_REFUSED` so a short
   restart doesn't fail the whole test. Individual tests may still
   flake — re-run them.

4. **Padmin OAuth token provisioning**
   Pasion's provisioning path stamps new delegated-auth users with
   `is_guest = true` in palpo, which blocks the `/_palpo/admin/*`
   endpoints. Separately, the padmin OAuth scope asks for `device:*`
   but pasion's sync-devices job doesn't land the resulting
   `user_devices` row into palpo in time. The tests apply two
   workarounds via `helpers/matrix-admin.ts` + `helpers/db.ts`:
   - `clearGuestFlag()` — `UPDATE users SET is_guest = false` in palpo
   - `upsertDevice(localpart, deviceId)` — MAS admin endpoint call to
     seed a stub device so `auth_by_delegated_token` has something to
     match against.
   If/when pasion's provisioning learns to do these itself the
   `prepareUserForAdmin` helper can drop them.

## Diagnosing a failure

- Each Playwright test writes trace + screenshot to
  `test-results/<test>/` on failure. Use `npx playwright show-trace`.
- Pasion logs are usually the most informative: `docker logs examples-pasion-1`.
- Palpo logs surface admin auth failures: `docker logs examples-palpo-1`.
- Full DB access: `docker exec -i examples-postgres-1 psql -U palpo -d pasion`
  (swap `pasion` for `palpo` for the matrix side).

## Re-running from scratch

```bash
# wipe and rebuild the whole test state
npm run reset
npm test
```

`npm run reset` is idempotent — you can run it between spec iterations to
guarantee the "first user is alice" invariant, which several tests
depend on.
