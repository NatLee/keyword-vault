# Compiler Foundry optional server — access integration plan

> Required workflow: execute inline with executing-plans; no additional agents. User approved both GitHub/Google account binding/login and MCP/API OAuth. Root owns shared-core authoring and final integration.

**Contract:** ../../../../design-notes/2026-11-27-access-contract.md; stable `/tmp/access-kit-authoring/INTERFACES.md`, DTO examples and access-core plan. Provider and OAuth/MCP protocol are implemented only by the identical vendored `crates/access-kit` snapshot.

## Global constraints

- Existing password/session/bootstrap behavior and user IDs remain authoritative. Host re-reads disabled/role/current domain permissions for every invocation; TrustedPrincipal is an opaque server-only extension, never an HTTP header.
- Management and native consent use cookie identity, same-origin validation and immutable consent DTOs; bearer API/MCP never grants account-management authority.
- Secrets appear once after creation, remain only in component memory, and are cleared on close/navigation. Never store raw keys in localStorage, screenshots, fixtures committed to git or logs. Provider credentials from env; disabled buttons explain missing setup.
- Fixed registry and object schemas; no user-provided URI or in-process header impersonation. Shared dispatcher owns one durable audit attempt per real operation; host router reuse is uncounted.
- Follow existing theme,400/600 fonts,4px spacing, restrained radii, Lucide16/20/24 stroke1.5, responsive/accessibility/reduced-motion rules.
- Shared crate is draft until core author announces immutable hash; final copy and numbered schema migration must be byte-identical. New migrations only; no edits to existing0001.

## Task1: host adapter and failing security tests

**Files:** independent `server/Cargo.toml`, `server/src/{lib,auth,access,main}.rs`, `server/migrations/0001_host.sql` + byte-identical `0002_access.sql`, `server/tests/access.rs`; native static `server/web/{index.html,access.css,access.mjs}`; optional Dockerfile/server CI job. Root library Cargo.toml and its default graph remain unchanged.
**Deployment:** optional `cargo run --manifest-path server/Cargo.toml`; default bind127.0.0.1:8670, ES_SERVER_BIND/ES_SERVER_DATA_DIR/ES_SERVER_STATIC_DIR plus shared ACCESS_PUBLIC_URL/providers. Independent server requiresRust1.88; library retainsRust1.74 and only unicode-normalization runtime dependency.
**Host:** SQLite local users/passwords/disabled/admin/member + SHA256 session hashes, Argon2, bounded login limiter and atomic first-admin bootstrap (loopback-only while unconfigured); local admin may create/disable members and revoke sessions. Existing offline atlas stays usable separately.
**Registry:** compile_query, inspect_query, extract_terms all `queries:read`. Call existing Query::parse_with/clauses/terms directly with precise/default_slop options, input limit40,000 chars and unchanged parser depth128. Runtime parse errors controlled400 with bounded messages, never panic or fake success. Tests admin/member active, login/bootstrap/session lifecycle, invalid input, schema/audience/disable/revoke/count once. Native compiler bench and Connections/Consent routes use same server cookie identity.

- [x] Add integration tests first for real key→API success and exact same domain behavior; prove missing/forged authority, disabled user, revoke and wrong scope/audience reject. Verify own/admin audit visibility and count once.
- [x] Introduce weak-host adapter and OnceLock core handle; mount state-erased core router outside legacy origin middleware. Reuse host auth session primitives and fixed internal domain handlers.
- [x] Apply next-numbered copied access schema, vendor snapshot, add manifests/Docker/CI support. Preserve original tests and default library dependency promises.

## Task2: native connections/login/consent UI

- [x] Typed access client consuming agreed DTOs, native themed bindings/key/grant/usage/audit/client sections. Login provider state comes from real endpoint. Cookie session is never inferred from bearer.
- [x] Write meaningful UI tests first: one-time secret cleared, disabled provider, request audiences/scopes, revocation, UTC usage/data/error/empty and consent immutable form action approve/deny.
- [x] Native consent route fetches pending request by opaque ID, shows current account/client/scopes/exact resource and posts a plain same-origin form to `/oauth/consent/{id}`. Login return only local `/oauth/authorize` next.
- [x] API/MCP daily bars show real server totals and pending separately; filters7/30/90,transport,user(admin), cursor/outcome. Explicit empty/error states; no fake grants/connections/counts.

## Task3: full acceptance and artifacts

- [x] Refresh final immutable identical core crate and copied SQL; run core/host tests/build/lint and unchanged domain regressions. Record dependency snapshot hash.
- [x] Actual official rmcp3.12 client (2026-07-28 per-request metadata) discovers/calls domain tool; assert same permission checks and one count, immediate revoke/disable, current domain membership change.
- [x] Real local key create→API+MCP→usage increase→revoke→denied; provider/consent behavior via core's controlled mock tests, no claim of live GitHub/Google registration. Actual local UI screenshots after backend works; hide raw secret before capture.
- [x] Desktop1440/mobile390, keyboard/reduced-motion, source/computed style scan, no overflow/errors, screenshots/README links/decode and diff check. Update README/env/deployment/manual provider-registration notes.
- [x] Commit explicit host scope and report SHA/tests/snapshot/server PIDs to root. No push/merge.

## Review focus

Private/group/disabled authority cannot survive to bearer calls. Core extensions are not serializable, legacy middleware cannot accidentally block OAuth token endpoints, consent cannot mutate scopes/redirect and refresh cannot expand audience. Secret closure clears DOM and memory; screenshots show server-generated usage and opaque prefixes only. Public provider callback success requires deployment registration; disclose local mock provenance.

## Completion evidence

See [access verification](../../access-verification.md). Docker builds/runtime smoke are delegated to the root reviewer; no claim of Docker completion is made here. Production provider registration remains a deployment check. Final snapshot source/schema hashes are recorded in the verification document.
