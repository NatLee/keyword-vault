# Keyword Vault: expression workspace

The interface is built around the work of collecting and reusing search syntax.
It uses a ruled, warm paper canvas, an ink masthead, amber operators, a numbered
horizontal tool navigation, and indexed expression records. Search and scope
filters stay next to the library. The dashboard introduces the query language
and connects to the real collection; counts, activity, and recently updated
expressions continue to use the API. Login has its own split composition.

Desktop navigation becomes an expandable mobile menu with an accessible label,
Escape dismissal, and the same single theme control. A skip link leads directly
to the workspace. Cards allow their contents to shrink, and long expressions
wrap or truncate without forcing the mobile viewport wider. The unused Inter
font dependency was removed; Noto Sans TC remains self-hosted.

The editable cover is `docs/images/cover.svg`. All 14 PNG screenshots were
recaptured from the working application using only its built-in fictional demo
data. The old cover artwork was removed. `docs/capture-ui.py` reproduces browser
checks and screenshots on an isolated demo instance; it requires Playwright
and `KV_QA_PASSWORD`, and accepts `KV_QA_URL`, `KV_QA_USER`, and an optional
`PLAYWRIGHT_CHROMIUM_PATH`.

## Correctness fixes

- **First-admin race:** two concurrent setup requests previously both returned
  200 and created administrators. A conditional SQLite INSERT checks the empty
  user table and writes in one atomic statement. Exactly one request now
  succeeds; the other returns 409. The regression was observed failing before
  the change and passing after it.
- **Pagination overflow:** a very large page multiplied by page size panicked in
  debug builds and could wrap in release builds. Checked multiplication now
  returns 400 for an unrepresentable offset. The largest valid offset remains
  accepted. A failing-before/passing-after integration test covers both cases.
- **Development login:** Vite's string proxy shorthand rewrote Host, causing
  same-origin login POSTs to return 403. An explicit proxy configuration retains
  the original Host. The full UI login flow now passes through that proxy.
- **Parser boundaries:** the vendored parser now groups both sides of a filtered
  query, and rejects more than 128 levels of parentheses with a structured error.
  Its source and 14 regression tests match the standalone library. Expanded
  combinations are validated before insertion and during cascades, so a depth
  failure rolls back the base update and leaves the existing combination valid.
- **Mobile overflow:** long recent expressions could enlarge the dashboard grid
  to over 1500px on a 390px screen. Explicit single-column grid sizing and
  shrinkable cards fix the cause. Browser checks cover every main route at 390px.

The syntax guide now describes proximity as phrase slop rather than a count of
Chinese characters. Backup documentation accounts for SQLite WAL state.

## Verification

Command output is in [verification/](verification/). Rust format and Clippy
(warnings denied) pass. The API passes 12 unit and 10 integration tests; the
vendored parser passes 14 integration and one documentation test. The frontend
passes nine tests and its production TypeScript/Vite build.

Chromium checks at 1440×1000 and 390×844 cover login through the proxy, query
search, favorite changes, validated creation, invalid syntax feedback, six
mobile routes without horizontal overflow, navigation, theme switching and
logout. There are no browser page errors. See
[verification/browser.log](verification/browser.log).

Native builds and a local development server were tested. Docker image build
and external search-engine execution were not part of this local run. Existing
integration coverage also exercises sharing/group visibility, history,
combination propagation and cycle rejection, CSV rollback/round-trip and origin
rejection. This audit is scoped to reviewed code paths and does not establish a
complete application security certification.

## Public-source review

Reachable Git history and tracked source were scanned for key markers, common
provider tokens, credential assignments, local user paths, and environment or
database assets. The sole historical credential-pattern match was
`src/web/src/pages/Login.test.tsx`, which contains explicitly fake test inputs.
No real secret or personal-data candidate was found. Demo people and content
are fictional. Screenshots and logs contain no session tokens, passwords or
local user paths. Heuristic scanning cannot identify every encoded secret.
