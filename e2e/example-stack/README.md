# Example Stack Smoke Suite

Playwright smoke coverage for the committed `examples/compose.yml` stack.
This suite stays under the main repository test workspace; there is no
separate `examples/e2e` package anymore.

## Purpose

- Verify the example stack still boots cleanly from Docker Compose.
- Cover example-stack-specific behavior that the main `padmin` suite does not:
  - the first registered user becomes the Matrix admin
  - the admin can install an appservice through `padmin`
  - Element can complete the Pasion SSO flow against the local stack

The broader `padmin` regression suite remains under `e2e/` and uses
`playwright.config.ts`. This smoke suite uses
`playwright.example-stack.config.ts`.

## Commands

```bash
# wipe the example stack and rebuild it in a test-friendly state
npm run stack:reset

# run the smoke suite against the example stack
npm run test:example-stack

# one-shot reset + run
npm run test:example-stack:fresh

# optional debugging helpers
npm run test:example-stack:headed
npm run test:example-stack:debug
npm run test:example-stack:report
```

## Layout

```
e2e/
├── example-stack/
│   ├── README.md
│   └── smoke.spec.ts
├── fixtures/
│   └── example-stack/
│       ├── compose.override.yml
│       └── pasion.test.yaml
├── helpers/
│   ├── element.ts
│   ├── matrix-admin.ts
│   ├── padmin-browser.ts
│   ├── pasion-browser.ts
│   ├── postgres.ts
│   └── services.ts
└── scripts/
    └── reset-example-stack.mjs
```

## Stack Reset Behavior

`e2e/scripts/reset-example-stack.mjs` is a wrapper around:

```bash
docker compose -f examples/compose.yml -f e2e/fixtures/example-stack/compose.override.yml
```

It does three things:

1. Brings the stack down with `-v` when requested.
2. Swaps the committed `examples/pasion.yaml` for a test override using
   `blackhole` email/SMS.
3. Applies the existing queue-table workaround for Pasion's cold-boot crash loop.

## Known Constraints

- The suite assumes a fresh stack. Use `npm run stack:reset` or
  `npm run test:example-stack:fresh`.
- The example users are intentionally fixed as `alice`, `bob`, and `carol`
  because the smoke checks assert the "first user becomes admin" invariant.
