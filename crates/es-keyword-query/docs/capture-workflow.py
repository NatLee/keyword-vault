"""Capture the real finite atlas workflow and render its separate repo cover.

python -m pip install playwright && python -m playwright install chromium
Optional: PLAYWRIGHT_CHANNEL=chrome ES_QA_URL=http://127.0.0.1:15379/docs/index.html
Without ES_QA_URL, the checked-in offline page is used. No cluster is contacted.
"""
import json
import os
from pathlib import Path
from playwright.sync_api import sync_playwright, expect

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'docs/images'


def settle(page):
    page.evaluate('document.fonts.ready')
    page.evaluate('() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))')


with sync_playwright() as p:
    browser = p.chromium.launch(headless=True, channel=os.environ.get('PLAYWRIGHT_CHANNEL'))
    context = browser.new_context(viewport={'width': 1440, 'height': 1100}, reduced_motion='reduce', accept_downloads=True)
    context.add_init_script("localStorage.setItem('atlas-theme', 'light')")
    page = context.new_page()
    errors = []
    page.on('pageerror', lambda error: errors.append(str(error)))
    page.goto(os.environ.get('ES_QA_URL', (ROOT / 'docs/index.html').as_uri()), wait_until='networkidle')
    specimen = page.evaluate('window.ATLAS[0]')
    assert specimen['id'] == 'discovery' and specimen['input'] == '(tea|coffee)&shop!ad'
    page.locator('#specimens button').nth(0).click()
    page.get_by_role('tab', name='00 Source').click()
    expect(page.locator('#stage-panel pre')).to_have_text(specimen['input'])

    def shot(number, section):
        expect(page.locator('#source')).to_have_text(specimen['input'])
        section.scroll_into_view_if_needed()
        page.evaluate('(y) => window.scrollTo(0, y - 16)', section.bounding_box()['y'] + page.evaluate('scrollY'))
        settle(page)
        if number == 3:
            section.screenshot(path=str(OUT / f'workflow-{number:02}.png'), animations='disabled')
        else:
            page.screenshot(path=str(OUT / f'workflow-{number:02}.png'), animations='disabled')

    shot(1, page.locator('.atlas'))
    page.get_by_role('tab', name='02 Structure').click()
    page.get_by_role('button', name='Clause 5: ad excluded', exact=True).focus()
    page.keyboard.press('Enter')
    expect(page.locator('.clause-label')).to_have_text('ad')
    expect(page.locator('.clause-meta')).to_contain_text('yes · direct !')
    page.locator('#mode').select_option('1')
    expect(page.locator('#compiled')).to_have_text(specimen['modes'][1]['query_string'])
    shot(2, page.locator('.atlas'))
    page.get_by_role('tab', name='03 Compile').click()
    expect(page.locator('#stage-panel pre')).to_have_text(specimen['modes'][1]['query_string'])
    # Add the actual second specimen as a filter, preserving the selected source.
    page.locator('#filter').select_option('1')
    expected = specimen['modes'][1]['filtered'][1]
    expect(page.locator('#compiled')).to_have_text(expected)
    with page.expect_download() as download:
        page.get_by_role('button', name='Export inspection').click()
    payload = json.loads(Path(download.value.path()).read_text())
    assert payload['query_string'] == expected
    assert payload['input'] == specimen['input']
    shot(3, page.locator('.output-workbench'))
    # Exercise the same final state at narrow width and keyboard stage navigation.
    page.set_viewport_size({'width': 390, 'height': 844})
    assert page.evaluate('document.documentElement.scrollWidth <= innerWidth')
    page.get_by_role('tab', name='03 Compile').focus()
    page.keyboard.press('Home')
    expect(page.get_by_role('tab', name='00 Source')).to_be_focused()
    expect(page.locator('#compiled')).to_have_text(expected)
    assert not errors, errors
    context.close()
    cover = browser.new_page(viewport={'width': 1440, 'height': 720}, device_scale_factor=1)
    cover.goto((OUT / 'src/cover.html').as_uri(), wait_until='networkidle')
    settle(cover)
    cover.screenshot(path=str(OUT / 'cover.png'))
    print(json.dumps({'source': specimen['input'], 'mode': 'raw', 'filter': 'specimen 02 / phrase slop', 'download': 'query_string and source verified', 'mobile_width': 390, 'keyboard': 'node Enter / stage Home passed', 'page_errors': errors, 'captures': 3, 'cover': '1440x720'}, indent=2))
    browser.close()
