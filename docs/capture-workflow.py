"""Capture a real subject → draft → private copy workflow on isolated demo data.

KV_QA_URL=http://127.0.0.1:18686 KV_QA_PASSWORD=... python docs/capture-workflow.py
Optional: KV_QA_USER=demo PLAYWRIGHT_CHANNEL=chrome
Requires built frontend/server and the built-in fictional demo, including keyword #1.
Creates one private copy, verifies its persisted data and deletes it in finally.
Never run this script against a production database. Credentials are not captured.
"""
import json
import os
from pathlib import Path
from playwright.sync_api import sync_playwright, expect

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'docs/images'
BASE = os.environ.get('KV_QA_URL', 'http://127.0.0.1:18686').rstrip('/')
EXPRESSION = '(珍珠奶茶|波霸奶茶)&評價!(業配|廣告)'


def settle(page):
    page.evaluate('document.fonts.ready')
    page.evaluate('() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))')


with sync_playwright() as p:
    browser = p.chromium.launch(headless=True, channel=os.environ.get('PLAYWRIGHT_CHANNEL'))
    context = browser.new_context(viewport={'width': 1440, 'height': 1100}, reduced_motion='reduce')
    page = context.new_page()
    errors = []
    page.on('pageerror', lambda error: errors.append(str(error)))
    page.goto(BASE, wait_until='networkidle')
    page.get_by_label('帳號', exact=True).fill(os.environ.get('KV_QA_USER', 'demo'))
    page.get_by_label('密碼', exact=True).fill(os.environ['KV_QA_PASSWORD'])
    page.get_by_role('button', name='登入', exact=True).click()
    expect(page.get_by_role('heading', name='語法工作台')).to_be_visible()
    page.get_by_role('radio', name='淺色', exact=True).click()
    original = context.request.get(BASE + '/api/keywords/1').json()
    assert original['theme'] == '珍珠奶茶' and '手搖飲' in original['tags']
    count_before = context.request.get(BASE + '/api/keywords?page_size=1').json()['total']
    page.get_by_role('link', name='語法庫', exact=False).first.click()
    page.locator('.subject-index').get_by_role('button', name='手搖飲', exact=False).click()
    expect(page.locator('.catalogue-selected')).to_contain_text('手搖飲')
    source_link = page.get_by_role('link', name='拆解與試寫：珍珠奶茶', exact=True)
    expect(source_link).to_be_visible()
    assert source_link.get_attribute('href') == '/inspect?keyword=1'
    settle(page)
    page.screenshot(path=str(OUT / 'workflow-01.png'), animations='disabled')
    source_link.click()
    draft = page.get_by_label('草稿語法')
    expect(draft).to_have_value(original['expression'])
    draft.fill(EXPRESSION)
    node = page.get_by_role('button', name='節點 7：廣告', exact=True)
    expect(node).to_be_visible()
    node.focus()
    page.keyboard.press('Enter')
    expect(page.get_by_text('位於排除群組內')).to_be_visible()
    check = context.request.post(BASE + '/api/tools/check', data={'expression': EXPRESSION, 'precise': True}).json()
    assert check['valid']
    expect(page.locator('.inspection-result code')).to_have_text(check['query_string'])
    expect(page.get_by_text('從 #1 載入', exact=True)).to_be_visible()
    page.evaluate('() => window.scrollTo(0, document.querySelector(".inspection-workspace").getBoundingClientRect().top + scrollY - 16)')
    settle(page)
    page.screenshot(path=str(OUT / 'workflow-02.png'), animations='disabled')
    saved_id = None
    try:
        page.get_by_label('新語法名稱').fill('珍珠奶茶 · 去除廣告的口碑草稿')
        with page.expect_response(lambda response: response.url == BASE + '/api/keywords' and response.request.method == 'POST') as response:
            page.get_by_role('button', name='另存私人語法').click()
        assert response.value.ok
        saved_id = response.value.json()['id']
        link = page.get_by_role('link', name='開啟新語法 →')
        expect(link).to_be_visible()
        assert link.get_attribute('href') == f'/keywords/{saved_id}'
        saved = context.request.get(BASE + f'/api/keywords/{saved_id}').json()
        assert saved['expression'] == EXPRESSION and saved['visibility'] == 'private'
        assert saved['tags'] == original['tags']
        assert context.request.get(BASE + '/api/keywords/1').json() == original
        page.locator('.inspection-save').scroll_into_view_if_needed()
        settle(page)
        page.screenshot(path=str(OUT / 'workflow-03.png'), animations='disabled')
        # The same loaded draft must fit the mobile document; also verify the
        # saved copy remains accessible through the actual success link.
        page.set_viewport_size({'width': 390, 'height': 844})
        assert page.evaluate('document.documentElement.scrollWidth <= innerWidth')
        expect(draft).to_have_value(EXPRESSION)
        link.click()
        expect(page.get_by_role('heading', name=saved['theme'], exact=False)).to_be_visible()
        assert page.evaluate('document.documentElement.scrollWidth <= innerWidth')
    finally:
        if saved_id is not None:
            assert context.request.delete(BASE + f'/api/keywords/{saved_id}').ok
            assert context.request.get(BASE + f'/api/keywords/{saved_id}').status == 404
    assert context.request.get(BASE + '/api/keywords/1').json() == original
    assert context.request.get(BASE + '/api/keywords?page_size=1').json()['total'] == count_before
    assert not errors, errors
    context.close()
    cover = browser.new_page(viewport={'width': 1440, 'height': 720}, device_scale_factor=1)
    cover.goto((OUT / 'src/cover.html').as_uri(), wait_until='networkidle')
    settle(cover)
    cover.screenshot(path=str(OUT / 'cover.png'))
    print(json.dumps({'source': '#1 珍珠奶茶', 'subject': '手搖飲', 'persisted_private_copy': 'expression, visibility and tags verified', 'original': 'unchanged', 'cleanup': 'copy deleted; total restored', 'mobile_width': 390, 'keyboard': 'node Enter passed', 'page_errors': errors, 'captures': 3, 'cover': '1440x720'}, ensure_ascii=False, indent=2))
    browser.close()
