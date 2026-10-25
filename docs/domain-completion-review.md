# Domain semantic completion review

Reviewed 2026-10-23 against the actual migration source files, beyond compilation and passing fixtures. Scope: 18 routes (dashboard, five user/known-user routes, two room routes, ten Pasion routes). The remaining routes and shared application components belong to the coordinator's separate review.

## Source API and component comparison

`domain-api-source-map.json` records each source page/component and its business service calls. All qualified source-domain calls have an active implementation. Deliberate replacements are cached-list helpers with explicit server refresh and `delete_all_user_devices` with repeat-first-page bounded deletion, preserving correct deletion when pages shift and reporting failures. The Pasion pages import the same dedicated service functions directly.

- `components/user_account_data.rs` has only GET and JSON display; no source write operation exists. The active dedicated `user_extras::account_data` preserves it.
- `components/user_rate_limits.rs` GET, POST override and DELETE defaults are preserved; invalid integers now fail before a request.
- `components/experimental_features.rs` GET and per-feature PUT remain. Actual server absence is surfaced; no success response is invented.
- Source user devices, joined rooms and whois are preserved. Whois walks all returned devices/sessions/connections and displays IP/time/agent. Browser fixtures assert nested content; the real Matrix suite asserts the live endpoint.
- Source user server notice validates nonempty text, sends m.text and clears after success. The active detail panel does the same, with confirmation and visible failures.
- Source CSV import uses pasted text and actually requires `id`; its UI claim that `displayname` alone works is inaccurate. The active parser preserves valid input and additionally validates quoting, duplicates, booleans and finite batch size. Source export formula protection, `user_id` header, seconds and `users_export.csv` have been restored.
- Source room list type chips filter the current server page, not the entire backend. This exact scope is retained. Bulk block/delete, state-event sorting and partial failure feedback were discovered as omissions during this independent review and implemented.
- Source Pasion notification channels expose only channel/configured and Refresh/Retry: no detail, edit or health management endpoint exists. Active channel rendering preserves the actual capability. Connector health independently displays provider/homeserver/status/error.
- Source provider form covers 25 fields. Token auth methods (12), discovery (3), PKCE (3), response (blank/query/form_post), and backchannel logout (3) are preserved, including untouched unknown existing enum values; secret keep/clear/new, config read-only, i32/parameter/claims validation are covered by regression tests and real create/PATCH/secret-clear/enable/disable/delete tests.
- Source audit operation filtering and source dashboard enabled unstable features/latency were discovered as omissions and restored in this independent review.
- Real Pasion revealed an existing source personal-token protocol defect. The active typed request uses required actor_user_id/human_name/scope and reads access_token. Actual creation/regeneration/revocation now passes.

## Original assertions and redesign

Business capabilities and protocol assertions are retained through semantic roles and labelled panels/records rather than the old table/tab/placeholder DOM. Creation currently navigates to the new user's detail, with Users navigation/search tested; Cancel returns directly to Users. These must be represented as equivalent behavior with changed DOM/navigation in the global assertion map, never as claims that unmodified old selectors passed.

`admin-scenarios.spec.ts:164` expects `/pasion/policy-data`, but the captured source router and all source Pasion modules contain no such route or implementation. This is stale test drift, not a missing migrated business operation. Do not invent a policy API/page to satisfy it. Source audit operation filtering at line 143 is real and is now implemented.

## Evidence and cleanup decision

Final independently reviewed debug bundle was rebuilt and all expanded checks executed successfully:

- `scripts/browser-domains.cjs`: 10 groups, including enabled dashboard features, audit operation filter, all list/cursor recovery, controlled account mutations and personal token wire behavior.
- `scripts/browser-domains-writes.cjs`: 10 groups, including whois devices/sessions/connections, read-only account data, actual CSV download filename/header/formula protection, creation password strength/validation/availability/Cancel, partial import, user and Pasion request bodies.
- `scripts/browser-rooms-writes.cjs`: 5 groups, including source type chips/state-events sorting, selected-page bulk block partial failure and bulk delete, all room writes and validation.
- `scripts/browser-real-domains.cjs`: real Pasion PKCE authentication and nine business groups passed, including actual account/provider/email/identity/token CRUD and isolated template publication. No real response was intercepted.

Result JSONs are in `D:/BuildCache/padmin-glory/{browser-domains-results,browser-domains-writes-results,browser-rooms-writes-results,real-pasion-domain-results}.json`. Palpo backend source independently confirms `last_membership_at` is the sorting parameter while the returned timestamp field is `last_membership_ts`.

**Domain review conclusion: the captured domain reference source may be removed.** All 18 assigned domain routes and the domain components' source business capabilities have active implementations and independent behavioral evidence. The final coordinator still owns compilation/unit tests, real Matrix verification and shared shell/i18n/lifecycle acceptance; deletion should be performed once that global review is also complete. An unavailable experimental-features endpoint in the configured Palpo server remains an explicit supported-error result, not an omitted UI operation or a fabricated success.
