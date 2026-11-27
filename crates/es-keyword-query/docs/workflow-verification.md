# Cover and actual query workflow

Verified on 2026-11-27. This change updates documentation and visual assets only;
no parser or browser behavior changed.

## What the images represent

- `images/cover.png`: a separate 1440 × 720 Compiler Foundry illustration, editable
  at [images/src/cover.html](images/src/cover.html). It uses the existing bundled
  Barlow Condensed font. It is a README cover, not a configured GitHub Social Preview.
- `workflow-01.png` and `workflow-02.png`: actual 1440 × 1100 browser viewports,
  scrolled to the explorer. The same `Discovery` expression persists across frames.
- `workflow-03.png`: an element crop of the actual release workbench at the same
  1440 px browser width. No UI labels, state or results are added to the images.
- The previous light, dark and mobile captures remain in the README's optional gallery.

## Reproduce

```sh
python -m pip install playwright
python -m playwright install chromium
python docs/capture-workflow.py
# Or use an installed Chrome:
PLAYWRIGHT_CHANNEL=chrome python docs/capture-workflow.py
```

The script opens `docs/index.html` offline by default. To exercise a local server,
set `ES_QA_URL=http://127.0.0.1:15379/docs/index.html`. It also renders the cover
from its local HTML source. All fonts are local; no search cluster is contacted.

The three actions are: select Discovery and Source; inspect the directly excluded
`ad` node and switch to Raw; then open Compile, choose Phrase distance as the
filter, and export. The source remains `(tea|coffee)&shop!ad`.

## Evidence

Commands run with the existing Python/Playwright environment and Chrome:

- `PLAYWRIGHT_CHANNEL=chrome python docs/capture-workflow.py` — passed all UI,
  downloaded JSON, source identity, 390 px document width and keyboard assertions;
  zero page errors. [Recorded output](verification/workflow-browser.log).
- `cargo run --example inspect -- '(tea|coffee)&shop!ad'` — the actual Rust output
  is `("tea" OR "coffee") AND "shop" NOT "ad"`, matching the cover's literal example.
- `cargo run --example build_atlas -- --check` — checked-in Rust-generated data is current.
- `python -m py_compile docs/capture-workflow.py` — passed.
- `git diff --check` — passed.
- All README local image/file targets exist; all documentation PNGs decode, and
  the cover is exactly 1440 × 720.
- Inspected every workflow image and both full-size and 720 px cover previews.
  The illustration, precise/raw distinction, direct exclusion and grouped filter
  remain readable. These are compilation examples; they make no claim about matches
  against a particular engine, analyzer or dataset.

The exported `query_string` is checked against the displayed filtered output,
`(("apple pie"~5) AND bakery) AND ((tea OR coffee) AND shop NOT ad)`, and its source
is checked against Discovery. Desktop capture uses a fresh browser context and
waits for fonts and two animation frames. The narrow-width check exercises the
same final state; Enter selects a node and Home returns to the first stage tab.
