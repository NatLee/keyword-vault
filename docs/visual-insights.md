# Query anatomy implementation plan

Goal: help a team understand saved expressions and safely refine a draft before saving a reusable query.

The new anatomy desk uses warm specimen-paper, amber numbered branch rails and typographic term tiles. A left-hand saved-query drawer leads into an editable draft, real API validation, a selectable grouping diagram and exact compiled output. It is visually distinct from the compiler atlas. Saved-query details and the existing editor also expose the same anatomy view.

Architecture: the vendored Rust parser exports a flat preorder clause inspection. `syntax::compile` serializes that structure into the authenticated check endpoint and keyword details. React renders the supplied parent relationships; it does not parse or guess query semantics. The inspector loads only queries visible to the current user through existing APIs, edits a local draft, and saves a named private copy. Existing queries remain editable using the established editor.

- [x] Test Rust hierarchy and authenticated API inspection, including invalid syntax.
- [x] Mirror the parser source and tests; include clause data in check/detail responses.
- [x] Build a reusable accessible anatomy component with selection and bounded large-tree rendering.
- [x] Add the inspector page with saved-query search, draft validation, reset, copy and save-as-private actions; add entry points from navigation, dashboard and detail.
- [x] Test draft loading, invalid validation, stale-response behavior and private save payload.
- [x] Run Rust/vendor tests, format/Clippy, frontend tests and production build.
- [x] Exercise real Chrome desktop/mobile/dark and saved-query workflows against an isolated demo database; update screenshots, README and validation notes.
- [x] Review, commit, push and create a PR without merging.

The diagram shows explicit grouping and operator sequence, never assumed precedence or match counts. A pending/failed validation must hide previous results and disable saving. Large expressions remain compilable; their visual preview is bounded and clearly labeled.

## Verification results

- Parser source and all 16 regression tests match the standalone crate exactly.
- Rust API: 12 unit tests and 11 integration tests pass; format and Clippy pass. The inspection regression verifies authentication, parent relationships, joiners, exclusion, slop and invalid-input behavior.
- Frontend: 15 tests pass, including a failing-first stale-validation regression, node inspection, bounded rendering, private-copy save payload and invalid-draft save blocking. Production TypeScript/Vite build passes.
- Actual Chrome against an isolated demo database: saved draft loading, invalid syntax clearing, private-copy creation, original preservation, editor structure preview and persisted edits pass. Test records are deleted afterwards.
- Chrome widths 320, 390, 768, 1024, 1101 and 1440 pass without document overflow. Light/dark/mobile and navigation pass without page errors.
- Reproduce with `docs/capture-insights.py`; it requires demo credentials in environment variables and deliberately writes only to the isolated demo instance.

A source-to-draft load uses the existing permission-checked keyword endpoint; unavailable queries produce the normal error and cannot reveal a previous query's anatomy. No new unauthenticated endpoint or search-engine connection was introduced.
