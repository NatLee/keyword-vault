# Keyword Vault consolidation verification — 2026-11-28

The product implementation follows [the accepted specification](superpowers/specs/2026-11-28-keyword-consolidation-design.md) and [execution plan](superpowers/plans/2026-11-28-keyword-consolidation.md). The old sibling repository was read-only throughout this implementation. No production database, real provider credentials, push, PR, merge or archive action was used.

## Compiler and workspace

Ran from the repository root with `CARGO_BUILD_JOBS=2`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` and a shared disposable `CARGO_TARGET_DIR`:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python3 scripts/check-compiler.py
cargo run -p es-keyword-query --locked --example build_atlas -- --check
```

All passed: **104 Rust tests including the compiler doctest** across access-kit, compiler and Vault; five CLI cases verify normalized output, raw output, slop, syntax exit1 and unknown-option exit2. The atlas remains the original finite eight-example reference. Standalone compiler testing actually ran inside official `rust:1.74`, copying only the compiler outside the workspace, and passed all16 integration cases. The compiler's sole runtime dependency remains `unicode-normalization`; jieba, SQLx, Axum and account handling remain in the host.

New behavior was tested failing-first at its owning boundaries. The access tests use the real current MCP SDK to call compiler compatibility, workbench and Chinese analysis tools. Existing visibility, audience, scope, disabled/pending account and durable audit behavior still pass. `queries:read` is separately selectable and not automatically granted to old credentials or preselected in the new-key form.

Nine migration integration tests and a transaction verification unit test exercise real disposable legacy/Vault SQLite schemas and hashes: read-only dry-run; report secrecy; explicit ID/reviewer/identity mapping; supported Argon2 login; pending/rejected/disabled/OAuth-only preservation; username/provider/hash/state conflicts; source/destination drift and report tamper; no overwrite of existing backup; verified byte-identical backup; complete transaction rollback after each of11 deliberate account/identity/reviewer mutations; repeated apply rejection; excluded tokens/sessions; canonical actual schema and outstanding WAL rejection; CLI defaults to dry-run without initializing the unrelated application DB. Fixture WAL data is explicitly checkpointed before immutable snapshot checks. [Migration operations](compiler-migration.md) document required offline conditions and recovery.

## Frontend and actual browser

```bash
cd src/web
pnpm exec vitest run --maxWorkers=2
pnpm build
```

All **49 tests in14 files** and the TypeScript/Vite production build passed. Tests cover stale results, exact JSON export, source-only saves, explicit grouped filter application, raw-mode analysis restriction, token identity display and first-login token handling. Existing account, registration, access ledger, compare and query views remain covered.

```bash
python docs/verify-consolidation.py --binary /absolute/path/to/keyword-vault
```

The new harness starts its own loopback service, temporary SQLite DB, custom dictionary and isolated headless Google Chrome. It performs real UI interactions and checks downloaded JSON against source/filter/slop, verifies a private save through the actual API, confirms analysis does not mutate source, toggles raw mode, applies the filter explicitly and captures desktop/mobile in both themes. **No page errors or horizontal overflow** occurred. Synthetic password fields are empty in screenshots. A small mobile checkbox-label alignment issue found during visual inspection was fixed and the harness rerun successfully.

- [Desktop workbench, light](images/consolidation-workbench-desktop-light.png)
- [Desktop workbench, dark](images/consolidation-workbench-desktop-dark.png)
- [Mobile workbench, light](images/consolidation-workbench-mobile-light.png)
- [Mobile workbench, dark](images/consolidation-workbench-mobile-dark.png)
- [Direct loopback first-login layout](images/consolidation-first-login.png)

The connected CUA browser inventory was empty. The authorized repository Playwright regression/capture workflow therefore used an isolated test browser, without connecting to the user's signed-in browser.

## Docker and nginx

All three final targets built successfully from the root workspace:

```bash
docker build --target runtime -t keyword-vault:consolidation-runtime .
docker build --target frontend -t keyword-vault:consolidation-frontend .
docker build --target standalone -t keyword-vault:consolidation-standalone .
python docs/verify-consolidation-compose.py
```

Final reviewed runtime image SHA begins `43d4f7527589`, frontend `163473c46947`, standalone `39236210367d`. The harness used a random project name, loopback port, independent volume and generated setup token; it removed its own containers/network/volume afterwards. Compose configuration and `nginx -t` passed. Actual nginx traffic verified missing/wrong setup token and spoofed forwarding are denied, correct token creates one administrator, subsequent setup closes, and pending registration cannot log in.

All five compatibility/workbench/analysis operations succeeded over both bearer REST and MCP through nginx; durable usage showed exactly five successes/attempts per transport. Backend recreation preserved the session, saved private query and usage while nginx stayed running. The API runs as `app` and exposes no host port. Revoking the credential immediately returned401, and runtime logs contained no generated password, setup token or API key. The first harness attempt correctly caught a fixture mistake (using an already signed-in client to register); a separate anonymous test context fixed the harness, without product changes.

The standalone image also passed `docs/smoke-test.sh` with a generated `ACCESS_SETUP_TOKEN`, covering protected setup, cookie flags, invalid syntax, keyword/composition creation, CSV export, cross-origin denial and logout. CI uses the same generated-token path. Existing Vault migration preservation is covered by workspace integration tests; compiler account migration is covered separately by the offline fixtures above.

Actual protected first-login screenshots through nginx:

- [Desktop, light](images/consolidation-first-login-desktop-light.png)
- [Desktop, dark](images/consolidation-first-login-desktop-dark.png)
- [Mobile, light](images/consolidation-first-login-mobile-light.png)
- [Mobile, dark](images/consolidation-first-login-mobile-dark.png)

## Scope and limits

Local jieba previews are dictionary-based and do not claim equivalence to Elasticsearch. No Elasticsearch cluster or live OAuth provider is required or was contacted. Configured custom dictionary bytes contribute to the displayed hash. The source-only save deliberately does not persist preview raw/slop settings; saving source with an unspecified `~` continues to use the server default on later compilation.

Historical verification documents retain their original dates and counts. The current commands and evidence here supersede their former vendored paths and unguarded remote first-admin setup assumptions. After final acceptance, the consolidation was released in [PR #9](https://github.com/NatLee/keyword-vault/pull/9); the old repository was redirected and archived. Its source and commit history remain available, while this repository owns current maintenance.


## Independent release-review fixes

The reviewer reproduced two defects against the original consolidation checkpoint. Both were reproduced by new failing regressions before fixes:

- Migration receipts did not prove that actual SQLite schema was supported. A destination trigger could enable an imported disabled account. The importer now compares full actual schema with a fresh canonical database built from the embedded migrations, rejecting extra triggers/columns and missing indexes on both sides. Tests also invoke the real CLI and confirm rejection occurs before any report, backup or database changes. A separate precommit snapshot check compares complete existing and imported user, identity and registration rows; eleven mutations to role, status, disabled/password flags/hash, identity ownership/provider/subject, existing users and reviewer mappings all fail and roll back through the actual commit guard.
- Compatibility parser errors used `domain_denied` instead of the old host's `compile_error`. Authenticated HTTP and the actual MCP SDK now verify exact machine code and original English message for all three operations: `compile_query`, `inspect_query`, `extract_terms`. Other domain errors retain their existing behavior.

After fixes, all104 workspace Rust tests, workspace Clippy with warnings denied, formatting and full baseline diff whitespace checks pass. The compiler evidence log and API Cargo manifest trailing blank lines were removed. Frontend product code did not change in this review pass; the preceding49 tests, production build and screenshot checks remain applicable.


Rebuilt the final runtime and standalone images and reran the entire isolated Compose harness. It additionally verifies all three legacy errors over both HTTP and MCP, each with `compile_error` and the exact English message, yielding three failures plus five successes per transport (eight attempts each). Setup guards, pending denial, persistence, revocation and secret-free logs still pass. The standalone smoke passed again. All owned test containers and volumes were removed.

A disposable Docker CLI replay built a supported legacy source with a disabled OAuth-only account and a fresh Vault destination. Valid dry-run succeeded; adding the reviewer's `AFTER INSERT users` trigger caused both new dry-run and prior-report apply to fail before report/backup creation. Restoring the exact clean destination allowed apply, preserving disabled/member/active/password-disabled flags, provider subject, source bytes and byte-identical preimport backup. This independently exercises the release binary in addition to the automated native CLI regressions.
