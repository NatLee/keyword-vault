"""Exercise the unified workbench in an isolated, disposable local browser.

Requires built web/API, Python Playwright and Google Chrome. Never uses an
existing server/database. Captures desktop/mobile light/dark with no secrets.
"""
import argparse
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import urllib.request
from playwright.sync_api import sync_playwright, expect

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', type=Path, default=ROOT/'target/debug/keyword-vault')
parser.add_argument('--output-dir', type=Path, default=ROOT/'docs/images')
args = parser.parse_args()
args.output_dir.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix='vault-consolidation-') as temporary:
    with socket.socket() as address:
        address.bind(('127.0.0.1', 0))
        port = address.getsockname()[1]
    base = f'http://127.0.0.1:{port}'
    custom = Path(temporary)/'dictionary.txt'
    custom.write_text('光儲整合 1000000 nz\n', encoding='utf-8')
    env = {**os.environ, 'KV_BIND':f'127.0.0.1:{port}', 'KV_DATA_DIR':temporary,
           'KV_DATABASE_PATH':str(Path(temporary)/'vault.db'), 'KV_STATIC_DIR':str(ROOT/'src/web/dist'),
           'KV_PUBLIC_URL':base, 'ACCESS_PUBLIC_URL':base, 'KV_COOKIE_SECURE':'false',
           'KV_SEGMENTATION_DICT':str(custom)}
    env.pop('ACCESS_SETUP_TOKEN', None)
    with open(Path(temporary)/'server.log', 'w') as log:
        server = subprocess.Popen([str(args.binary.resolve())], cwd=ROOT, env=env, stdout=log, stderr=log)
        try:
            for _ in range(100):
                if server.poll() is not None:
                    raise RuntimeError(Path(log.name).read_text())
                try:
                    urllib.request.urlopen(base+'/api/health', timeout=1).close()
                    break
                except OSError:
                    time.sleep(.1)
            with sync_playwright() as p:
                browser = p.chromium.launch(headless=True, channel='chrome')
                context = browser.new_context(viewport={'width':1440,'height':1100}, reduced_motion='reduce')
                page = context.new_page()
                errors = []
                page.on('pageerror', lambda error: errors.append(str(error)))
                page.goto(base+'/first-login', wait_until='networkidle')
                expect(page.get_by_role('button', name='建立管理員', exact=True)).to_be_visible()
                page.evaluate('document.fonts.ready')
                page.screenshot(path=str(args.output_dir/'consolidation-first-login.png'), full_page=True)
                response = context.request.post(base+'/api/auth/setup', data={'username':'qa.admin','password':'Disposable browser test 2026'})
                assert response.ok, response.text()
                page.goto(base+'/inspect', wait_until='networkidle')
                source = '(珍珠奶茶|光儲整合)&(台灣 高鐵 ~)!廣告'
                page.get_by_label('草稿語法', exact=True).fill(source)
                page.get_by_label('篩選語法（選填）', exact=True).fill('臺灣|台灣')
                page.get_by_label('預設片語 slop', exact=True).fill('8')
                export = page.get_by_role('button', name='匯出檢視 JSON', exact=True)
                expect(export).to_be_enabled()
                with page.expect_download() as download:
                    export.click()
                artifact = json.loads(Path(download.value.path()).read_text())
                assert artifact['expression'] == source
                assert artifact['options'] == {'precise':True, 'default_slop':8}
                assert artifact['filter_expression'] == '臺灣|台灣'
                assert artifact['query_string'].startswith('("臺灣" OR "台灣") AND (')
                assert '~8' in artifact['query_string']
                page.get_by_role('button', name='預覽中文斷詞', exact=True).click()
                expect(page.locator('.analysis-token').filter(has_text='光儲整合')).to_be_visible()
                assert page.get_by_label('草稿語法', exact=True).input_value() == source
                page.get_by_label('新語法名稱', exact=True).fill('整合工作台驗證')
                page.get_by_role('button', name='另存來源語法', exact=True).click()
                saved = page.get_by_role('link', name='開啟新語法 →', exact=True)
                expect(saved).to_be_visible()
                record = context.request.get(base+'/api'+saved.get_attribute('href')).json()
                assert record['expression'] == source and record['visibility'] == 'private'
                page.get_by_label('精確片語模式', exact=True).uncheck()
                expect(page.get_by_role('button', name='預覽中文斷詞', exact=True)).to_be_disabled()
                expect(page.locator('.analysis-leaf')).to_have_count(0)
                page.get_by_label('精確片語模式', exact=True).check()
                expect(export).to_be_enabled()
                page.get_by_role('button', name='套用篩選至草稿', exact=True).click()
                expect(page.get_by_label('草稿語法', exact=True)).to_have_value('(臺灣|台灣)&('+source+')')
                expect(page.get_by_label('篩選語法（選填）', exact=True)).to_have_value('')
                # Restore a rich preview for the visual evidence, independently of saved data.
                page.get_by_label('草稿語法', exact=True).fill(source)
                page.get_by_label('篩選語法（選填）', exact=True).fill('臺灣|台灣')
                expect(export).to_be_enabled()
                page.get_by_role('button', name='預覽中文斷詞', exact=True).click()
                expect(page.locator('.analysis-token').filter(has_text='光儲整合')).to_be_visible()
                for theme in ['light','dark']:
                    page.set_viewport_size({'width':1440,'height':1100})
                    page.get_by_role('radio', name='淺色' if theme=='light' else '深色', exact=True).click()
                    for width, name in [(1440,'desktop'),(390,'mobile')]:
                        page.set_viewport_size({'width':width,'height':1100 if width==1440 else 844})
                        page.evaluate('document.fonts.ready')
                        assert page.evaluate('document.documentElement.scrollWidth <= innerWidth'), (theme,width)
                        path = args.output_dir/f'consolidation-workbench-{name}-{theme}.png'
                        page.screenshot(path=str(path), full_page=True, animations='disabled')
                        print(f'Screenshot: {path}', flush=True)
                assert not errors, errors
                print(json.dumps({'checks':['guarded first-login layout','actual inspection/filter/slop JSON download',
                    'custom Traditional dictionary tokens','source-only private save','raw disables analysis',
                    'explicit grouped filter apply','desktop/mobile light/dark without overflow'],
                    'page_errors':errors}, ensure_ascii=False), flush=True)
                browser.close()
        finally:
            server.terminate()
            try:
                server.wait(timeout=10)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait()
