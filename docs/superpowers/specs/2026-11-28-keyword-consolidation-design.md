# Keyword Vault consolidation and Traditional Chinese analysis

Status: implementation authorized by the user's product-evolution request. This project spec implements section 1 and the builder-review decisions in `../../../design-notes/2026-11-28-product-evolution-design.md` (relative to the repository root's parent). The old es-keyword-query repository stays read-only during implementation; root handles its redirect and archive only after final verification. History is never deleted.

## Product and module boundaries

Keyword Vault becomes the single maintained application, repository, account system and deployment. A root Cargo workspace contains `src/api`, `crates/access-kit` and `crates/es-keyword-query`. The latter keeps package name `es-keyword-query`, version `0.1.0`, Rust `1.74`, the existing public types/methods and its sole runtime dependency `unicode-normalization`. Do not add fields to its public `Options` or change the meaning of `Query::terms()`.

Move the identical vendored compiler into `crates/es-keyword-query`; bring the original examples, documentation, atlas, license and regression tests into this repository. Keep the library usable without a database, HTTP server, authentication or Chinese dictionary. Preserve `cargo run -p es-keyword-query --example inspect -- '<expression>' --raw --slop=5` and the original exit codes (valid 0, syntax error 1, arguments error 2). The finite atlas remains clearly labeled as eight offline examples.

The one public deployment remains nginx on `KV_PORT` (default 8686), an internal nonroot Vault process and the existing `keyword-vault-data` volume. Keep the standalone image, API/OAuth/MCP routes, canonical origin checks, existing accounts, data and registration behavior. Add no mandatory Elasticsearch service.

## Compiler workbench

Keep the existing `/tools/check` and `inspect_expression` result/error contracts. Add `/api/tools/inspect` and the `inspect_workbench` API/MCP operation, scope `expressions:read`, for structured inspection with composition. Input is `{expression, filter_expression?, precise?, default_slop?}`. Both source strings are bounded by 50,000 Unicode scalar values, and their combined length is also at most 50,000. Slop is an unsigned 32-bit integer. Omitted precise is true; omitted slop uses `KV_DEFAULT_SLOP` (default 20).

A successful response is `{valid:true, inspection:{schema_version:1, expression, filter_expression, options:{precise,default_slop}, query, filter, query_string}}`. `query` and optional `filter` each contain the existing `query_string`, `terms`, `clauses`. `query_string` is the final artifact: use `Query::filtered_by` when a nonblank filter is supplied, grouping both sides. No inferred Boolean precedence is introduced. Invalid syntax returns `{valid:false,error:{source:"expression"|"filter",message,position}}`, using the parser's Chinese message and zero-based character position.

Inspector gains precise/raw controls, per-request slop, an optional filter editor and JSON export. Every control participates in the stale-response guard. Clear stale compiled results, structure and analysis immediately after edits and after errors. Export only the current successful artifact as `keyword-inspection.json`; it must exactly match the visible final query. The source and filter keep separate clause trees/labels.

Saving remains explicit: "另存來源語法" saves only the authored source as a private ordinary record, with inherited tags. Show "預覽設定與篩選條件不會存入來源語法。未指定距離的 ~ 仍使用伺服器預設值。" The optional "套用篩選至草稿" action replaces the draft with `(<filter>)&(<source>)`, clears the filter and requests fresh inspection. Changing preview options or exporting never mutates the database or original record. Existing 5,000-character save and 50,000-character expanded/inspection limits remain.

## Explicit compiler compatibility

Preserve cookie `POST /api/compiler/inspect` and bearer/MCP `compile_query`, `inspect_query`, `extract_terms`, scope `queries:read`, in a separate compatibility module. Inputs deny unknown fields and are exactly `{expression,precise?,default_slop?}`, at most 40,000 characters. Outputs and English HTTP error behavior match the original server. Do not route these directly to the Chinese `valid:false` response. Existing `keywords:read` and `expressions:read` grants do not implicitly acquire `queries:read`.

Connections shows the new scope. Both protocols use the existing access-kit invocation pipeline, preserving per-invocation scope/audience/account checks and one audit attempt. Cookie-only connection management and current keyword visibility remain authoritative. No duplicate authentication store or legacy issuer acceptance is added.

## Traditional Chinese preview

Use pinned `jieba-rs = 0.11.0` in the Vault API only. Bundle Jieba's Traditional-friendly `extra_dict/dict.txt.big` from upstream revision `67fa2e36e72f69d9134b8a1037b83fbb070b9775`, preserve its MIT license and record the downloaded SHA-256. Add a small documented Taiwan/domain dictionary. Initialize one immutable analyzer at startup; CPU work runs outside async reactor tasks and uses a maximum of two concurrent analysis jobs.

`KV_SEGMENTATION_DICT` optionally selects a deployment-owned UTF-8 custom dictionary file. Limit it to 1 MiB, 10,000 nonblank entries, words of 1–100 Unicode scalars and positive frequency up to 1,000,000,000; rows are `word frequency [tag]`. Reject duplicates, invalid frequency, whitespace in words, malformed UTF-8 and invalid files at startup. Do not accept filesystem paths from HTTP/MCP. No per-user dictionary is persisted or shared implicitly.

Add authenticated cookie `POST /api/tools/analyze` and bearer/MCP `analyze_expression` with scope `expressions:read`. Input is `{expression,mode?:"normal"|"search",precise?:boolean}`; default mode is normal and precise is true. Limit input to 10,000 Unicode scalars. `precise:false` produces an actionable validation error: raw Lucene fields/wildcards are not natural language. Parse first; analyze only term/proximity `Clause.text`, preserving clause indices, direct negation and source order. Group nodes and operators are never segmented. Reject unresolved combination references rather than treating `#123` as text; normal Vault combinations already arrive expanded.

Result is `{engine:"jieba-rs",engine_version:"0.11.0",mode,dictionary:{id,sha256},offset_unit:"unicode_scalar",leaves:[{clause,text,tokens:[{text,start,end}]}]}`. Offsets are end-exclusive Unicode scalar positions within that normalized, unescaped leaf text, not raw-expression offsets or UTF-16/UTF-8 bytes. Preserve punctuation/whitespace in offsets but omit punctuation-only/whitespace-only tokens from suggestions. Do not truncate silently: fail if output exceeds 20,000 tokens. Normal mode uses HMM disabled for deterministic dictionary-driven output; search mode may contain overlapping subwords and is labeled accordingly.

Inspector displays tokens and dictionary identity with "本機中文斷詞預覽；不代表 Elasticsearch 分析結果。" Analysis never changes `Query::terms`, compiled output or stored source. Choosing an individual token copies it as escaped literal text for composing a new draft. AND/OR generation, if exposed, is a separate explicit action that creates a fresh draft from user-selected tokens; it never overwrites exclusions, groups or proximity automatically. No default simplified/traditional conversion. Local analysis is useful without an Elasticsearch cluster.

## Bootstrap parity

Carry the compiler's setup-token protection into Vault. `ACCESS_SETUP_TOKEN` is either unset or 32–512 nonblank characters, compared in constant time and redacted from Debug/logs. Real loopback peers may bootstrap without it; nonloopback peers (including nginx) require it. Forwarded headers never make a connection loopback. State returns only required/configured booleans. The first-login UI sends the token in the JSON body and never stores it or places it in a URL. Invalid configuration fails startup. Existing populated installations require no token.

Add `keyword-vault bootstrap <username>` reading its password only from stdin, validating it and atomically creating the first administrator without a listener. Remote setup and CLI remain closed once any user exists. Keep the current atomic empty-database insertion guard and registration lifecycle.

## Offline account migration

Existing Vault data is primary. The compiler's `0001_host.sql` and Vault's `0001_init.sql` differ, so database-file replacement is forbidden. The importer accepts two explicit existing paths and never calls application startup/migrations on the source. It requires stopped services and no outstanding WAL data. It opens the source read-only and checks schema/migration fingerprints against the supported compiler schema, plus SQLite integrity/foreign-key checks.

CLI: `keyword-vault import-compiler --source <snapshot.db> --destination <vault.db> --report <report.json>` is dry-run by default. It generates a versioned artifact containing source/destination canonical paths, schema and content fingerprints, proposed source-to-target user ID map, account states, identity metadata, conflicts, excluded entity counts and no password hashes/tokens. `--apply <report.json> --backup <new-backup.db>` requires an unchanged reviewed artifact and revalidates both databases. Backup is created exclusively, verified byte/content-identical before writes, and never overwritten. Hold a destination write transaction while rechecking and inserting; all writes commit together or none do.

Import users, their Argon2id v19 password hashes (m=8192..65536 KiB, t=1..6, p=1..4; 16–64 byte output, salt present, exactly these three parameters), role, disabled/account status/password-login flags, timestamps, provider identities and registration state. Preserve pending/rejected/disabled restrictions. Reject unknown hash algorithms/parameters or account statuses in dry-run. An OAuth-only account may have no usable password and keeps password login disabled. Username matching follows SQLite NOCASE; any username, provider-subject or user-provider collision is a reported blocker. No automatic merge, renaming, role promotion or identity attachment to existing users. Operators can resolve conflicts in a separately prepared source snapshot and rerun dry-run; the importer never silently skips accounts.

Map all user IDs explicitly, including registration reviewer IDs; dangling reviewers or identity rows block. Allocate fresh destination identity/registration identifiers rather than overwrite existing ones. Do not import sessions, PATs, OAuth clients/consents/codes/grants/tokens, provider flows or usage/audit records. Report these exclusions and require fresh login/relink/credentials at the new origin. Keep source history/backups for provenance. Reapplying cannot duplicate users: fingerprints or conflicts reject it. Existing destination keywords, relationships, visibility, groups and registration policy are unchanged.

## Verification and rollout

Run compiler examples/regression/atlas checks, an isolated Rust 1.74 library build, all workspace unit/integration/access-kit tests, frontend tests/build, formatting/clippy, Docker image/Compose validation and isolated nginx smoke. Pin library MSRV without forcing the application's Rust 1.88+ dependencies into its standalone build. Use `CARGO_BUILD_JOBS=2`, debug info disabled and frontend workers at most two in this shared workspace.

Meaningful new tests cover unchanged compiler contracts; OR-containing filter grouping; stale requests; export equality; save separation; Chinese scalar offsets/emoji/fullwidth/escaped operators; dictionary bounds; raw/reference rejection; API/MCP scopes and audit counts; token/bootstrap protection through nginx; migration fingerprints, backup, hashes, statuses, identity conflicts and rollback on disposable legacy fixtures. No production databases or real OAuth providers are used.

Capture actual desktop/mobile light/dark workbench and first-login screenshots with synthetic data. Document real commands/results and limits in `docs/consolidation-verification.md`. Update README/API/deployment docs and provenance. Commit locally after focused checks. Parent performs independent branch review and owns push/PR/merge plus the old repository's redirect/archive after all acceptance checks pass.

## Primary references

- https://docs.rs/jieba-rs/0.11.0/jieba_rs/struct.Jieba.html
- https://github.com/fxsjy/jieba#其他词典
- https://www.elastic.co/docs/reference/query-languages/query-dsl/query-dsl-query-string-query
- https://www.elastic.co/docs/manage-data/data-store/text-analysis/index-search-analysis
- https://www.elastic.co/docs/reference/elasticsearch/plugins/analysis-smartcn
