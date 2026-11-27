"""Exercise the anatomy desk against an isolated demo instance.

Creates, edits and deletes one private test query. Never use a production database.
KV_QA_URL=http://127.0.0.1:8686 KV_QA_PASSWORD=... python docs/capture-insights.py
Optional: PLAYWRIGHT_CHANNEL=chrome
"""
import json, os
from pathlib import Path
from playwright.sync_api import sync_playwright, expect
ROOT = Path(__file__).resolve().parents[1]
BASE = os.environ.get('KV_QA_URL', 'http://127.0.0.1:8686')
with sync_playwright() as p:
    browser = p.chromium.launch(headless=True, channel=os.environ.get('PLAYWRIGHT_CHANNEL'))
    context = browser.new_context(viewport={'width':1440,'height':1100})
    page = context.new_page(); errors = []; checks = []
    page.on('pageerror', lambda error: errors.append(str(error)))
    page.goto(BASE)
    page.get_by_label('帳號', exact=True).fill(os.environ.get('KV_QA_USER','demo'))
    page.get_by_label('密碼', exact=True).fill(os.environ['KV_QA_PASSWORD'])
    page.get_by_role('button', name='登入', exact=True).click()
    expect(page.get_by_role('heading', name='語法工作台')).to_be_visible()
    page.goto(BASE+'/inspect?keyword=1')
    original = context.request.get(BASE+'/api/keywords/1').json()
    expect(page.get_by_label('草稿語法')).to_have_value(original['expression'])
    checks.append('load saved expression through authenticated API')
    expression = '(珍珠奶茶|波霸奶茶)&評價!(業配|廣告)'
    draft = page.get_by_label('草稿語法'); draft.fill(expression)
    expect(page.get_by_role('button', name='節點 7：廣告', exact=True)).to_be_visible()
    page.get_by_role('button', name='節點 7：廣告', exact=True).click()
    expect(page.get_by_text('位於排除群組內')).to_be_visible()
    draft.fill('a&&b')
    expect(page.get_by_text('不可有連續的邏輯運算子（第 3 個字元）')).to_be_visible()
    expect(page.get_by_role('button',name='另存來源語法')).to_be_disabled()
    expect(page.locator('.anatomy')).to_have_count(0)
    draft.fill(expression)
    expect(page.get_by_role('button', name='節點 7：廣告', exact=True)).to_be_visible()
    page.get_by_label('新語法名稱').fill('解剖工作台 · 測試語法')
    page.get_by_role('button',name='另存來源語法').click()
    link = page.get_by_role('link',name='開啟新語法 →');expect(link).to_be_visible()
    saved_id = link.get_attribute('href').rsplit('/',1)[-1]
    try:
        saved = context.request.get(BASE+'/api/keywords/'+saved_id).json()
        assert saved['visibility'] == 'private' and saved['expression'] == expression
        assert context.request.get(BASE+'/api/keywords/1').json()['expression'] == original['expression']
        checks.append('invalid draft hides anatomy; private copy preserves original')
        link.click()
        expect(page.locator('.anatomy')).to_be_visible()
        page.get_by_role('button',name='編輯',exact=True).click()
        page.get_by_label('關鍵字語法',exact=True).fill(expression+'&台北')
        page.get_by_text('拆開查看語法結構').click()
        expect(page.get_by_role('dialog').locator('.anatomy')).to_be_visible()
        page.get_by_role('button',name='儲存',exact=True).click()
        expect(page.get_by_role('dialog')).to_have_count(0)
        expect(page.get_by_role('button',name='節點 8：台北',exact=True)).to_be_visible()
        assert context.request.get(BASE+'/api/keywords/'+saved_id).json()['expression'] == expression+'&台北'
        checks.append('existing editor previews and persists actual saved-query edits')
    finally:
        assert context.request.delete(BASE+'/api/keywords/'+saved_id).ok
    page.goto(BASE+'/inspect')
    expect(page.locator('.anatomy')).to_be_visible()
    page.get_by_role('button', name='節點 5：括號群組', exact=True).click()
    page.get_by_role('radio',name='淺色',exact=True).click()
    page.evaluate('document.fonts.ready')
    page.screenshot(path=str(ROOT/'docs/images/screenshot-inspector-light.png'),full_page=True)
    page.get_by_role('radio',name='深色',exact=True).click()
    page.screenshot(path=str(ROOT/'docs/images/screenshot-inspector-dark.png'),full_page=True)
    for width in [320,390,768,1024,1101,1440]:
        page.set_viewport_size({'width':width,'height':900})
        assert page.evaluate('document.documentElement.scrollWidth <= innerWidth'),width
    page.set_viewport_size({'width':390,'height':844})
    page.get_by_role('button',name='開啟選單').click()
    page.get_by_role('radio',name='淺色',exact=True).click()
    page.get_by_role('link',name='語法解剖',exact=False).click()
    page.screenshot(path=str(ROOT/'docs/images/screenshot-inspector-mobile.png'),full_page=True)
    checks.append('desktop/mobile/dark, node selection and responsive navigation')
    assert not errors,errors
    print(json.dumps({'checks':checks,'page_errors':errors},ensure_ascii=False,indent=2))
    browser.close()
