# Query Cabinet / 關鍵字典藏室

The catalogue now follows the domain: teams collect expressions, group them by subject, combine references, and refine private working copies. The original keyword-platform provided permission scopes, tagged records, combination references and revision history; these are the basis of the new interface.

The dashboard is a physical four-drawer cabinet linked to the existing permission-scoped totals and filters. Subject folders link to real tags; the counts describe accessible records, never popularity or usage. A revision register and a randomly selected readable query replace the generic charts. The library lists every available subject, renders a two-column index-card catalogue, and offers a compact list without changing scope, search or sorting. Each record links directly to its existing inspector source.

The shared shell uses index tabs, ivory paper, warm ochre and ink. The login, detail record, live anatomy and private draft desk use the same visual language. The underlying authenticated API, parser and owner permissions are unchanged. Drafts remain private copies and never modify their source.

## Fonts

Noto Sans TC and Noto Serif TC Variable are bundled through Fontsource. The new `@fontsource-variable/noto-serif-tc` dependency is pinned to 5.3.1, imports `wght.css`, and includes the SIL Open Font License in its package. Official setup: https://fontsource.org/fonts/noto-serif-tc/use. No runtime font requests reach a third-party domain.

## Verification

- Frontend production build and 17 tests pass. New regressions cover switching card/list views without dropping the active scope, direct source-to-draft links, clearing only the tag, and browsing tags beyond the former twelve-tag cutoff.
- Actual Chrome against an isolated synthetic demo instance verifies login, search, favorites, keyword creation, invalid syntax, private-copy save, source preservation, edit persistence, navigation and logout.
- All 17 images use fresh browser contexts at the final viewport to capture light/dark desktop and mobile views after `document.fonts.ready` and two animation frames. Keyword details remain full-page captures so the compiled output, combinations and comments are included.
- 35 route/width combinations (320, 390, 768, 1024 and 1440 pixels) have no document overflow. Browser behavior checks and capture orchestration run from temporary files against a temporary demo database. No credentials or database files are tracked.
