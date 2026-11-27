# Themed motion frontend verification

2026-11-27. Frontend phase only; account linking (GitHub/Google) and OAuth for
MCP/API authorization follow a separate implementation plan.

## Source and behavior

- Original authored CSS and utility classes normalized across all frontend pages, including settings, login, dialogs, errors and mobile navigation.
- Fixed 4px spacing, 400/600 weights, flat surfaces, 0/4/8px ordinary radii. Automatic layout margins remain `auto`; chart/SVG geometry is not spacing.
- Domain sections retain their data contracts and existing component boundaries. No new API behavior, data simulation, animation dependency, notification or connection page.
- Selection transitions last 160–180ms. Reduced motion shows final content immediately; keyboard focus remains on the selected node.
- `python docs/audit-styles.py`: authored spacing, weights and prohibited effects pass. Computed style checks ignore the resolved value of authored `auto` margins and SVG geometry.

## Reproduction and results

- `pnpm --dir src/web test`: 6 files, 17 tests pass.
- `pnpm --dir src/web build`: TypeScript + Vite pass.
- `docs/capture-ui.py`: login, search, bookmark, create/delete, history, compare, invalid syntax, settings, responsive menu/theme/logout pass; no page errors.
- `docs/capture-insights.py`: authenticated load, invalid draft, actual private save/edit and cleanup pass.
- Computed audit covers dashboard, library, detail, compare, guide, settings and inspector at 1440/390 in both themes. Existing visually hidden labels now use zero-margin clipping rather than Tailwind's -1px margin.
- Workflow verifies the same #1 record/expression/tags, saves one private copy, verifies it and deletes it; original data and total are restored.

- `python docs/capture-workflow.py`: three actual workflow PNGs and editable-source cover refreshed. The cover is independent artwork, not a feature screenshot.
- `python docs/capture-motion.py`: real selection video, keyboard selection, reduced-motion final visibility, and computed source/spacing checks at 1440/390 in both themes.
- `python -m py_compile docs/capture-workflow.py docs/capture-motion.py docs/audit-styles.py`: pass.
- Local README targets, PNG decode and `git diff --check`: pass.

Use `PLAYWRIGHT_CHANNEL=chrome` when the installed Playwright cache belongs to a
different version. Videos contain actual browser actions and no login credentials.

## Manual review items and limits

- Confirm brand hue, density and reading order with the product owner; review font rendering on other operating systems.
- Existing semantic exclusion/validation/share colors retain text labels. Screen-reader announcement quality and full WCAG contrast coverage need a dedicated assistive-technology audit; this run is not a conformance claim.
- Workflow PNGs were visually inspected; the video was decoded and representative frames inspected. PNGs alone do not demonstrate motion.
- Screenshots use isolated fictional built-in demo data. Do not present demo counts or illustrated covers as production analytics.
- Root performs an independent code/visual review before integration. No merge was performed by this worker.
