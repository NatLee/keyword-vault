# Keyword Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Keyword Vault the maintained compiler/library/workbench product with Traditional Chinese analysis and a safe migration path.

**Architecture:** A Cargo workspace owns the unchanged lightweight compiler and shared access-kit. Vault supplies one authenticated application with explicit compiler compatibility operations, separate analysis and offline migration modules.

**Tech Stack:** Rust 2021, Axum 0.8, SQLx 0.8/SQLite, React/TypeScript/Vite, jieba-rs 0.11.0, nginx/Docker Compose.

**Spec:** [2026-11-28-keyword-consolidation-design.md](../specs/2026-11-28-keyword-consolidation-design.md)

## Global Constraints

- `es-keyword-query` remains version `0.1.0`, Rust `1.74`, with sole runtime dependency `unicode-normalization`; no changed public Options fields or Query::terms semantics.
- Keep `keyword-vault-data`, default port 8686, current private/group/public visibility, account IDs and API/MCP authorization.
- Inspection combined source/filter limit 50,000 characters; compatibility limit 40,000; saved source limit 5,000; analysis input limit 10,000 and output limit 20,000 tokens.
- Custom dictionary: 1 MiB, 10,000 entries, word length 1–100 scalars, frequency 1–1,000,000,000; HMM disabled; offsets are end-exclusive leaf-local Unicode scalars.
- Bootstrap token is 32–512 nonblank characters and required for nonloopback empty-install setup.
- Source expressions, existing compiler semantics, original repository history and production databases are never silently rewritten.
- Use `CARGO_BUILD_JOBS=2`, disabled debug info and frontend workers at most two. Coordinate image builds with parent. Local commits permitted; parent owns push/PR/archive.

## Review Focus

- Rapid source/filter/options switches must never export or save results for an older draft (Task 5).
- Chinese compatibility characters, astral characters and overlapping search tokens must not be mislabeled as raw-expression or JavaScript offsets (Tasks 6–7).
- Existing bearer grants must not gain compiler scope, and direct internal dispatch must count exactly one audited invocation (Task 4).
- Migration must reject drift, unsupported hashes/statuses, identity collisions and partial failure without changing either database or its verified backup (Tasks 9–10).
- nginx peers and forged forwarding headers must never bypass first-admin setup protection; populated databases still boot normally without a token (Task 8).

## File map and execution order

| Files | Responsibility |
|---|---|
| `Cargo.toml`, `Cargo.lock`, `crates/es-keyword-query/**` | Workspace and reusable compiler/examples/atlas |
| `src/api/src/workbench.rs`, `routes/tools.rs` | Structured compile/filter inspection |
| `src/api/src/compiler_compat.rs`, `access.rs` | Exact prior compiler API/MCP contracts |
| `src/api/src/analysis.rs`, `data/segmentation/**` | Bounded Traditional Chinese dictionary analysis |
| `src/web/src/components/InspectionControls.tsx`, `ChineseAnalysis.tsx`, `pages/Inspector.tsx` | Explicit preview/export/save/analysis interaction |
| `src/api/src/setup.rs`, existing auth/config/main files | Guarded bootstrap and password-stdin CLI |
| `src/api/src/migration.rs`, `tests/compiler_migration.rs` | Offline dry-run artifact, backup, transactional account import |
| `.github/workflows/ci.yml`, `Dockerfile`, Compose, docs | Reproducible build/deployment/provenance/evidence |

Tasks 1–4 establish backend interfaces; Task 5 consumes them. Tasks 6–7 add analysis without changing compilation. Task 8 restores setup parity. Tasks 9–10 are an independent offline utility. Task 11 verifies and documents the complete deliverable. Commit each coherent task after its stated checks; do not push.

### Task 1: Establish the workspace and single compiler source

**Files:** Create root `Cargo.toml`; move `src/api/vendor/es-keyword-query` to `crates/es-keyword-query`; copy original `examples/` and compiler docs into that crate; modify `src/api/Cargo.toml`, root lockfile, Dockerfile and CI paths.

**Interfaces:** Produces the unchanged `es_keyword_query::{Query,Options,Clause,Error}` under a versioned path dependency; workspace members are compiler, access-kit and API.

- [ ] Step 1: Record `git status`, old compiler source/test hashes and manifests; confirm the feature branch and preserve original repo read-only.
- [ ] Step 2: Move/copy only owned library assets, preserve license/provenance and existing examples. Consolidate lockfile and root profile settings; update builds to use workspace target paths.
- [ ] Step 3: Run `cargo test -p es-keyword-query` and `cargo run -p es-keyword-query --example build_atlas -- --check`. Expected: all existing tests pass and atlas matches byte-for-byte.
- [ ] Step 4: Run `cargo metadata --no-deps --format-version 1`; expected members exactly compiler/access-kit/API and one compiler path. Confirm core source hashes still equal original. Commit the structural change.

### Task 2: Preserve the CLI and standalone MSRV

**Files:** `crates/es-keyword-query/examples/{inspect.rs,support/mod.rs,build_atlas.rs}`, `scripts/check-compiler.sh`, `.github/workflows/ci.yml`.

**Interfaces:** Original CLI positional expression plus `--raw`/`--slop=N`; JSON valid/normalized/terms/clauses; exit codes 0/1/2. No application initialization.

- [ ] Step 1: Add a focused shell/CLI verification that checks `inspect '(a|b)&c'` output, `a&&b` exit 1 and an unknown flag exit 2.
- [ ] Step 2: Run the copied CLI commands and fix only moved-path assumptions. Assert `query_string == '("a" OR "b") AND "c"'` and raw `a*` remains `a*`.
- [ ] Step 3: Add a script that copies the compiler package into a temporary directory outside the workspace, removes inherited workspace context/lock, and runs its tests with Rust 1.74.0. Keep the package manifest self-contained.
- [ ] Step 4: Run the CLI script plus isolated MSRV check. Expected: all pass without Axum/SQLx/jieba in the compiler runtime dependency tree. Commit CLI/MSRV packaging evidence.

### Task 3: Add structured workbench inspection and grouped filters

**Files:** Create `src/api/src/workbench.rs`, `src/api/tests/workbench.rs`; modify `lib.rs`, `routes/tools.rs`, `routes/mod.rs`.

**Interfaces:** `InspectInput { expression:String, filter_expression:Option<String>, precise:Option<bool>, default_slop:Option<u32> }`; `workbench::inspect(input: InspectInput, default_slop:u32) -> AppResult<Value>` returns the spec's valid/error artifact. `/api/tools/inspect` requires `CurrentUser`.

- [ ] Step 1: Write tests `filter_groups_both_sides`, `options_do_not_mutate_source`, `filter_errors_identify_source`, `combined_length_is_bounded`. Core assertion:
  ```rust
  assert_eq!(result["inspection"]["query_string"], r#"("shop" OR "cafe") AND ("tea" OR "coffee")"#);
  assert_eq!(result["inspection"]["expression"], "tea|coffee");
  ```
- [ ] Step 2: Run `cargo test -p keyword-vault --test workbench`; expected initial failure because route/module is absent.
- [ ] Step 3: Implement one parse per source with existing library options, separate query/filter inspection and `filtered_by`. Keep old `/tools/check` unchanged.
- [ ] Step 4: Run the focused tests and existing API anatomy tests; expected success, exact options/limits/errors. Commit.

### Task 4: Register compiler compatibility and new workbench operation

**Files:** Create `src/api/src/compiler_compat.rs`; modify `access.rs`, routes, `src/web/src/pages/Access.tsx`, `src/web/src/lib/access.ts`; extend `src/api/tests/access.rs`.

**Interfaces:** `compiler_compat::Input` and `evaluate(input, operation, default_slop)->AppResult<Value>` match the old server; `operations()->Vec<DomainOperation>` declares the three `queries:read` tools. `inspect_workbench` consumes Task 3 input with `expressions:read`.

- [ ] Step 1: Add golden tests for `compile_query`, `extract_terms`, `inspect_query`, cookie `/api/compiler/inspect`, denied unknown fields, 40,000 limit and English syntax HTTP errors. Test old `inspect_expression` remains `valid:false` with Chinese error.
- [ ] Step 2: Run targeted access tests; expected failures for absent operations/scope.
- [ ] Step 3: Port the exact legacy compiler adapter; route it through existing access-kit invocation once. Expose `queries:read` in Connections without granting it automatically.
- [ ] Step 4: Verify HTTP plus official MCP SDK invocation, wrong scope/audience/disabled user rejection and exactly one daily attempt. Run `cargo test -p keyword-vault --test access`; commit.

### Task 5: Integrate options, composition, export and explicit save semantics

**Files:** Create `src/web/src/components/InspectionControls.tsx`; modify `pages/Inspector.tsx`, `api.ts`, `types.ts`, related CSS and `Inspector.test.tsx`.

**Interfaces:** `api.inspect(input: InspectInput):Promise<InspectionResult>` mirrors Task 3. `InspectionArtifact` is exported unchanged; existing `api.createKeyword` saves source only.

- [ ] Step 1: Write tests for raw/slop/filter controls, outdated requests, export data equality, saved original source and explicit apply-filter. Assert `createKeyword.expression === source` until the apply-filter button is clicked.
- [ ] Step 2: Run `pnpm test -- --maxWorkers=2 src/pages/Inspector.test.tsx`; expected new controls absent.
- [ ] Step 3: Add controls and snapshot all request fields in the stale guard; display source/filter structures separately. JSON download uses current response only. Apply-filter writes `(<filter>)&(<source>)` and clears filter. Use the exact save notice from the spec.
- [ ] Step 4: Run Inspector/QueryAnatomy/SyntaxCheck tests and `pnpm build`; expected all pass. Commit.

### Task 6: Add versioned Traditional Chinese analyzer and bounded dictionaries

**Files:** Create `src/api/src/analysis.rs`, `data/segmentation/{dict.txt.big,taiwan.txt,LICENSE,README.md}`; modify API Cargo manifest, config/state/lib; tests in analysis module and `tests/analysis.rs`.

**Interfaces:** `Analyzer::load(custom:Option<&Path>)->anyhow::Result<Analyzer>`; `analyze(&self, input:AnalysisInput)->AppResult<AnalysisResult>`. Input/result structs exactly match spec. `AppState` holds one immutable analyzer and a two-permit semaphore.

- [ ] Step 1: Test custom `臺積電` remains one token, `𠮷野家`/emoji offset roundtrip, fullwidth normalization, proximity/exclusion leaves, escaped operators, search overlap, raw/reference rejection and all dictionary bounds. Assertion for every returned token: `leaf.text.chars().skip(start).take(end-start).collect::<String>() == token.text`.
- [ ] Step 2: Run `cargo test -p keyword-vault analysis`; expected missing module/type.
- [ ] Step 3: Pin jieba-rs 0.11.0; fetch the exact upstream dictionary revision and license, record SHA-256, load Taiwan/custom entries after validation. Use HMM false and scalar offsets; reject over-limit output. Keep source text/compiler output unchanged.
- [ ] Step 4: Run focused tests and core regression suite; verify analysis dependency is absent from core manifest. Commit data provenance and implementation.

### Task 7: Expose analysis through API/MCP and the workbench

**Files:** Modify routes/tools/access, web api/types/Inspector; create `components/ChineseAnalysis.tsx` and its test; extend `tests/analysis.rs`, `tests/access.rs`.

**Interfaces:** `/api/tools/analyze`, `analyze_expression` scope `expressions:read`, `api.analyze(input):Promise<AnalysisResult>` consume Task 6; no caller paths or identity arguments.

- [ ] Step 1: Write HTTP/MCP unauthorized/scope/audit tests and UI tests proving preview leaves source unchanged, raw mode explanation, mode/identity display and stale result clearing.
- [ ] Step 2: Run targeted tests and observe missing route/UI failures.
- [ ] Step 3: Run CPU analysis in `spawn_blocking` under the two-permit bound. Add separate token panel and literal copy actions; do not auto-generate AND/OR or mutate source. Display the exact local-versus-ES notice.
- [ ] Step 4: Run analysis/access/UI tests and build; commit.

### Task 8: Carry deployment-token protection and offline bootstrap

**Files:** Create `src/api/src/setup.rs`; modify config/auth routes/main/serve, web FirstLogin/context/types, `.env.example`, Compose, tests and smoke scripts.

**Interfaces:** `SetupToken` redacted/constant-time validated config; peer comes from actual Axum `ConnectInfo<SocketAddr>`; `keyword-vault bootstrap <username>` reads password stdin and creates only the first administrator.

- [ ] Step 1: Port old server first-login/bootstrap fixtures. Test no token on remote fails; matching token succeeds; forged `X-Forwarded-For` cannot bypass; true loopback succeeds; simultaneous setup produces one admin; malformed configured token fails startup; existing users boot without token.
- [ ] Step 2: Run focused tests; expected remote setup currently succeeds and CLI is absent.
- [ ] Step 3: Port guard and secret handling, add actual socket connect info, booleans/UI field, password-stdin command without listener, Compose env. Update isolated smoke setup to provide its generated token.
- [ ] Step 4: Run first-login/registration/API suites and browser first-login tests; commit.

### Task 9: Build the read-only migration report

**Files:** Create `src/api/src/migration.rs`, `src/api/tests/compiler_migration.rs`; modify `lib.rs`, `main.rs`.

**Interfaces:** `MigrationReport` version 1; `migration::inspect(source:&Path,destination:&Path)->anyhow::Result<MigrationReport>`; CLI `import-compiler --source ... --destination ... --report ...` writes JSON exclusively, defaults to dry-run.

- [ ] Step 1: Create disposable compiler/Vault DB fixtures with schema checksums, Argon2 hashes, active/pending/rejected/disabled/OAuth-only accounts and identities. Assert dry-run leaves bytes/data unchanged, proposes complete ID mappings, excludes tokens and identifies NOCASE username/provider conflicts.
- [ ] Step 2: Run `cargo test -p keyword-vault --test compiler_migration`; expected absent module/command.
- [ ] Step 3: Validate canonical unequal paths, existing supported schemas, integrity, no uncheckpointed WAL, supported Argon2id v19 parameters/statuses and complete identity/reviewer references. Compute schema/content fingerprints; never run migrations or writable pragmas on source. Reports omit secrets.
- [ ] Step 4: Run tests including unsupported schema/hash/state and dangling references; commit.

### Task 10: Apply reviewed migration atomically with a verified backup

**Files:** `migration.rs`, `tests/compiler_migration.rs`, `main.rs`, `docs/compiler-migration.md`.

**Interfaces:** `migration::apply(report:&MigrationReport,backup:&Path)->anyhow::Result<MigrationReceipt>`; receipt includes committed ID map and verified backup digest, never credentials. CLI `import-compiler --apply <report.json> --backup <new-backup.db>`.

- [ ] Step 1: Tests: report tampering/drift refused, backup exists refused, partial insert rollback, unsupported identity conflicts refused, all source account restrictions preserved, existing destination rows untouched and reapply rejected. Assert source fingerprint remains equal and backup restores exact preimport destination.
- [ ] Step 2: Run migration tests; expected apply missing.
- [ ] Step 3: Recompute report against both selected DBs, compare immutable fingerprint/map/conflict fields, create/verify exclusive backup and lock destination writes while rechecking and inserting all rows. Copy only users/identities/registration state using explicit mapped IDs; exclude auth artifacts. On any error rollback and preserve backup/source.
- [ ] Step 4: Run full migration suite and document stop/checkpoint/backup/dry-run/apply/recovery steps plus explicit credential exclusions. Commit.

### Task 11: Complete integration, documentation and release evidence

**Files:** Dockerfile/Compose/nginx as needed, `.github/workflows/ci.yml`, `README.md`, `docs/api.md`, `docs/nginx-deployment.md`, `docs/consolidation-verification.md`, `docs/images/consolidation-*`, compiler atlas links.

**Interfaces:** One default Compose entry, workspace checks and documented compatible library/CLI/API/MCP; parent receives verified commit SHA plus remaining limits.

- [ ] Step 1: Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, CLI/atlas/MSRV checks, frontend tests with two workers and build. Expected all green; record actual counts/commands.
- [ ] Step 2: Coordinate Docker build with parent, build both images, run `docker compose config` and `nginx -t`; start a unique temporary project with fresh volumes/token. Verify real nginx first-login, registration, workbench/API/MCP, existing DB migration and preserved named-volume configuration.
- [ ] Step 3: Capture actual desktop/mobile light/dark workbench and first-login via the approved browser skill/tool; inspect screenshots and fix functional/layout issues. No real provider credentials or production data.
- [ ] Step 4: Update docs with exact commands, snapshots/dictionary provenance, save semantics, local analysis limitations, migration exclusions and future old-repo redirect. Record test evidence honestly; remove only owned temporary test resources.
- [ ] Step 5: Self-review spec coverage, run affected focused checks after fixes, commit locally and send parent the branch/head/evidence. Parent performs final independent review and owns old-repo redirect/archive after all checks. Do not push or archive here.

## Self-review record

All spec sections map to Tasks 1–11. Compatibility and workbench responses use separate adapters; analysis has its own contract and never changes core output. Migration inspector and apply consume the same versioned artifact and revalidate it. The five Review Focus conditions have tests assigned to their owning tasks. User/root authorization permits implementation after this self-review; no additional plan-approval gate is required.
