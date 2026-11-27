"""Verify the finite Rust-generated atlas and capture its real UI.

pip install playwright && playwright install chromium
Optional: PLAYWRIGHT_CHANNEL=chrome
"""
import json, os
from pathlib import Path
from playwright.sync_api import sync_playwright, expect
root = Path(__file__).resolve().parents[1]
with sync_playwright() as p:
    browser = p.chromium.launch(headless=True, channel=os.environ.get('PLAYWRIGHT_CHANNEL'), executable_path=os.environ.get('PLAYWRIGHT_CHROMIUM_PATH'))
    page = browser.new_page(viewport={'width': 1440, 'height': 1100}, accept_downloads=True)
    errors = []
    page.on('pageerror', lambda error: errors.append(str(error)))
    page.goto((root / 'docs/index.html').as_uri())
    data = page.evaluate('window.ATLAS')
    for i, item in enumerate(data):
        page.locator('#specimens button').nth(i).click()
        for mode in [0, 1]:
            page.locator('#mode').select_option(str(mode))
            value = item['modes'][mode]
            if value['valid']:
                expect(page.locator('#compiled')).to_have_text(value['query_string'])
                expect(page.locator('.node')).to_have_count(len(value['clauses']))
                page.locator('#filter').select_option('0')
                expect(page.locator('#compiled')).to_have_text(value['filtered'][0])
                page.locator('#filter').select_option('')
            else:
                expect(page.locator('#copy')).to_be_disabled()
                expect(page.locator('.error-pane')).to_contain_text(value['error']['message'])
    page.locator('#specimens button').nth(3).click()
    page.locator('#mode').select_option('0')
    page.locator('.node').last.click()
    expect(page.locator('.clause-label')).to_have_text('toy')
    page.get_by_role('tab', name='01 Normalize').click()
    expect(page.locator('#stage-panel')).to_contain_text(data[3]['modes'][0]['normalized'])
    page.get_by_role('tab', name='03 Compile').click()
    expect(page.locator('#stage-panel pre')).to_have_text(data[3]['modes'][0]['query_string'])
    page.get_by_role('tab', name='02 Structure').click()
    page.locator('.node').nth(6).click()
    with page.expect_download() as download:
        page.locator('#download').click()
    payload = json.loads(Path(download.value.path()).read_text())
    assert payload['query_string'] == data[3]['modes'][0]['query_string']
    for width in [320, 390, 768, 1024, 1440]:
        page.set_viewport_size({'width': width, 'height': 1100})
        assert page.evaluate('document.documentElement.scrollWidth <= innerWidth'), width
    page.close()

    # Capture in fresh contexts at the final viewport. Reusing the responsive
    # test page can make Chrome repeat a stale compositor tile in full-page PNGs.
    for filename, theme, viewport in [
        ('reference-desktop.png', 'dark', {'width': 1440, 'height': 1100}),
        ('reference-light.png', 'light', {'width': 1440, 'height': 1100}),
        ('reference-mobile.png', 'light', {'width': 390, 'height': 844}),
    ]:
        context = browser.new_context(viewport=viewport, reduced_motion='reduce')
        context.add_init_script(f"localStorage.setItem('atlas-theme', '{theme}')")
        capture = context.new_page()
        capture.on('pageerror', lambda error: errors.append(str(error)))
        capture.goto((root / 'docs/index.html').as_uri(), wait_until='networkidle')
        capture.locator('#specimens button').nth(3).click()
        capture.locator('.node').nth(6).click()
        expect(capture.locator('#compiled')).to_have_text(data[3]['modes'][0]['query_string'])
        expect(capture.locator('header')).to_have_count(1)
        expect(capture.locator('main')).to_have_count(1)
        expect(capture.locator('footer')).to_have_count(1)
        assert capture.locator('html').evaluate('(e) => e.classList.contains("light")') == (theme == 'light')
        capture.evaluate('document.fonts.ready')
        capture.evaluate('window.scrollTo(0, 0)')
        capture.mouse.move(0, 0)
        capture.evaluate('() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))')
        assert capture.evaluate('document.documentElement.scrollWidth <= innerWidth')
        capture.screenshot(path=str(root / 'docs/images' / filename), full_page=True, animations='disabled')
        context.close()
    assert not errors, errors
    print(json.dumps({'specimens': 8, 'modes': 2, 'filter_outputs': 14, 'stages': 'passed', 'node_selection': 'passed', 'export': 'passed', 'widths': [320, 390, 768, 1024, 1440], 'captures': '3 fresh contexts; fonts and paint settled', 'page_errors': errors}, indent=2))
    browser.close()
