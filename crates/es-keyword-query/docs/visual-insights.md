# Compiler Foundry

Goal: make the Rust compiler's structure explorable without pretending a static page runs arbitrary Rust code.

The foundry uses cobalt blueprint paper or a dark ink/blue drafting canvas, a technical specimen library, a selectable circuit tree and a four-step source → normalize → parse → compile production line. System type at 400/600 keeps the compiler stages and clauses readable without an extra display weight. Visitors choose finite verified specimens, switch precise/raw output, inspect a clause, and compose a grouped filter. A CLI example compiles arbitrary local input. The page explicitly labels its finite dataset and runs offline.

Architecture: `Query::clauses()` exposes a flat preorder view of the existing AST with parent indices, original joiners, direct negation, resolved phrase slop and compiled fragments. It does not infer Boolean precedence or execute a search. `examples/inspect.rs` emits JSON; `examples/build_atlas.rs` writes the verified fixture data used by the dependency-free browser page. JSON is a development dependency only. The same crate source is vendored into Keyword Vault.

- [x] Add failing Rust tests for hierarchy, joiners, exclusions, escaped terms and configured slop.
- [x] Implement the inspection API and CLI; mirror source/tests to Keyword Vault.
- [x] Generate fixture data from Rust; implement selectable specimens, stage rail, focused clauses, mode/filter controls, copy/export, and light/dark controls.
- [x] Run Rust tests, format, Clippy, rustdoc and fixture freshness checks.
- [x] Run real Chrome desktop/mobile/dark interactions and update screenshots and README.
- [x] Review the final diff, commit, push and create a PR without merging.

Validation must verify displayed outputs against generated Rust data. The graph represents written grouping and order, not search-engine precedence or result counts. The finite specimens keep the graph readable; arbitrary expressions and their complete inspection data are available through the CLI.

## Verification results

- Rust: 16 integration tests and one doctest pass. Format, Clippy with warnings denied and rustdoc pass.
- Generated fixture freshness: `cargo run --example build_atlas -- --check` passes and is part of CI.
- Chrome opens the actual page using `file://`: all 8 specimens, both output modes, 14 grouped filter outputs, stage switching, clause selection and JSON download pass.
- Widths 320, 390, 768, 1024 and 1440 have no document overflow. Dark/light screenshots are real captures from `docs/capture-reference.py`.
- No browser parser is duplicated. Arbitrary local input is supported by the Rust CLI; the browser is explicitly limited to the generated specimens.
- The CLI and example generator use a development-only JSON dependency. Library source/tests match the Keyword Vault vendored copy.

## Foundry revision

The original source is now a first-class stage. All four stage tabs support Left/Right, Home and End, with the process strip reflecting the selected stage. Live readouts describe the selected Rust record: character count, normalization status, clause count and compilation readiness. The invalid specimen blocks copying and filtering while still allowing its structured error inspection to be exported. No parser logic or fixture output changed.

Chrome checked all 64 specimen/mode/stage combinations, keyboard wrapping, process-strip state, and switching to an invalid specimen with an active filter. Existing capture verification checks all specimens, filter outputs, node selection, downloads and widths 320–1440. Screenshots use fresh contexts at their final viewport after local font loading and two animation frames.
