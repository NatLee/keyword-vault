> Historical notes for the first visual pass. The current compiler atlas and inspection API are documented in [visual-insights.md](visual-insights.md).

# Compiler field reference and parser audit

The project is a Rust library, so its public-facing surface is a developer
reference. The current `docs/index.html` is the Compiler Foundry: cobalt blueprint
paper, a black/cyan theme, a source → normalization → parsing → compilation rail,
and eight Rust-generated specimens. Selectable clauses and a release sheet make
the compiled output auditable. The
page explicitly identifies the examples as fixed reference material, not a
JavaScript compiler. No frontend framework or dependency was added.

The repository cover now uses the actual Compiler Foundry application capture.
`reference-desktop.png` and `reference-mobile.png` are actual Chromium captures
of the reference, produced by `docs/capture-reference.py`. The page works at
1440px and 390px widths with no horizontal overflow. Example switching and the
theme control were checked; the browser reported no page errors.

## Parser changes

- **Grouped filters:** `filtered_by` previously emitted `a OR b AND (query)`.
  The filter is now parenthesized too: `(a OR b) AND (query)`. A failing
  regression reproduced the missing grouping before the change.
- **Bounded recursion:** nested groups previously recursed without a bound.
  Parsing now reports `ErrorKind::NestingTooDeep` above 128 groups. Tests cover
  an accepted 128-level expression and a rejected 10,000-level input, protecting
  parsing and subsequent tree traversal from unbounded nesting.
- The three reference examples are compiled in a Rust regression test, keeping
  the displayed input/output pairs tied to actual library behavior.

## Validation

Captured command output is in [verification/](verification/). `cargo fmt --check`,
Clippy with warnings denied, 14 integration tests, one documentation test, and
`cargo doc --no-deps` passed. Browser output is in
[verification/browser.log](verification/browser.log).

No Elasticsearch or OpenSearch cluster was started: this crate compiles strings
and does not ship an engine client. Engine analyzers and query execution remain
outside this validation. The existing Rust 1.74 minimum is retained; validation
used the locally installed stable toolchain rather than an MSRV matrix.

## Public-source review

All reachable Git history and the tracked source were checked for private-key
markers, common provider tokens, environment/database/key assets, local user
paths, and hardcoded credential assignments. No candidate secret or private data
was found. This is a source review and pattern scan, not a guarantee against
credentials encoded in arbitrary forms. No account visibility was changed here.
