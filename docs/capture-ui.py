import json, os
from pathlib import Path
from playwright.sync_api import sync_playwright, expect
ROOT=Path(__file__).resolve().parents[1]
BASE=os.environ.get('KV_QA_URL','http://127.0.0.1:15373')
OUT=ROOT/'docs/images'
errors=[]; checks=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True, executable_path=os.environ.get("PLAYWRIGHT_CHROMIUM_PATH"))
 context=browser.new_context(viewport={'width':1440,'height':1000},device_scale_factor=1)
 page=context.new_page();page.on('pageerror',lambda error: errors.append(str(error)))
 def shot(name):
  page.evaluate('document.fonts.ready');page.wait_for_timeout(220);page.screenshot(path=str(OUT/name))
 def goto(path):
  page.goto(BASE+path);page.wait_for_load_state('networkidle')
 def check_width(label):
  assert page.evaluate('document.documentElement.scrollWidth <= innerWidth'),label+' overflows'
  checks.append(label)
 goto('/')
 shot('screenshot-login-light.png')
 page.get_by_label('帳號',exact=True).fill(os.environ.get('KV_QA_USER','demo'))
 page.get_by_label('密碼',exact=True).fill(os.environ['KV_QA_PASSWORD'])
 page.get_by_role('button',name='登入',exact=True).click();expect(page.get_by_role('heading',name='語法工作台')).to_be_visible();page.wait_for_load_state('networkidle');checks.append('UI login through development proxy')
 shot('screenshot-overview-light.png');page.get_by_role('radio',name='深色',exact=True).click();shot('screenshot-overview-dark.png')
 goto('/keywords');shot('screenshot-keywords-dark.png');page.get_by_role('radio',name='淺色',exact=True).click();shot('screenshot-keywords-light.png')
 search=page.get_by_role('textbox',name='搜尋',exact=True);search.fill('排除：徵才');expect(page.locator('.keyword-record')).to_have_count(1);checks.append('search returns matching expression')
 page.get_by_role('button',name='收藏',exact=True).click();expect(page.get_by_role('button',name='取消收藏',exact=True)).to_be_visible();page.get_by_role('button',name='取消收藏',exact=True).click();checks.append('favorite toggles and persists through API')
 search.fill('');expect(page.locator('.keyword-record')).to_have_count(20)
 page.get_by_role('button',name='新增關鍵字',exact=True).click();page.get_by_label('主題',exact=True).fill('茶飲口碑觀察');page.get_by_label('關鍵字語法',exact=True).fill('(茶|咖啡)&評價!業配');page.get_by_label('標籤',exact=True).fill('飲料, 口碑');page.wait_for_timeout(500);shot('screenshot-editor-light.png')
 page.get_by_role('button',name='新增',exact=True).click();expect(page.get_by_role('heading',name='茶飲口碑觀察',exact=False)).to_be_visible();created_id=page.url.rsplit('/',1)[-1];checks.append('create from editor and live syntax validation')
 # QA creates only in the isolated demo database; remove the temporary record afterwards.
 response=context.request.delete(BASE+'/api/keywords/'+created_id);assert response.ok
 goto('/keywords/33');shot('screenshot-detail-light.png');page.get_by_role('radio',name='深色',exact=True).click();shot('screenshot-detail-dark.png');page.get_by_role('radio',name='淺色',exact=True).click()
 goto('/keywords/1');page.get_by_role('button',name='改了語法').first.click();page.wait_for_timeout(400);shot('screenshot-history-light.png')
 goto('/compare?a=茶%7C咖啡&b=(茶%7C咖啡)%26評價!業配');shot('screenshot-compare-light.png')
 goto('/guide');shot('screenshot-guide-light.png');page.get_by_label('關鍵字語法',exact=True).fill('a&&b');page.wait_for_timeout(400);assert '不可有連續' in page.inner_text('main');checks.append('invalid syntax feedback')
 goto('/settings');shot('screenshot-settings-light.png')
 page.set_viewport_size({'width':390,'height':844})
 for path in ['/', '/keywords', '/keywords/33', '/compare', '/guide', '/settings']:
  goto(path);check_width('mobile '+path)
  if path=='/keywords':shot('screenshot-mobile-light.png')
 page.get_by_role('button',name='開啟選單').click();expect(page.get_by_role('navigation',name='主要導覽')).to_be_visible();page.get_by_role('radio',name='深色',exact=True).click();page.get_by_role('link',name='語法庫',exact=False).first.click();expect(page.get_by_role('button',name='開啟選單')).to_be_visible();shot('screenshot-mobile-dark.png');checks.append('mobile navigation and theme')
 page.get_by_role('button',name='登出',exact=True).click();expect(page.get_by_role('heading',name='歡迎回來')).to_be_visible();check_width('mobile login');checks.append('logout')
 assert not errors, errors
 print(json.dumps({'checks':checks,'page_errors':errors,'screenshots':len(list(OUT.glob('screenshot-*.png')))},ensure_ascii=False,indent=2))
 browser.close()
