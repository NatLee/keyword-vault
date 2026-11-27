"""Verify prebuilt consolidation images in an owned disposable Compose project.

Requires Docker Compose, Python Playwright and Chrome. Builds no images, never
uses the deployment's .env, containers or volume; always removes owned resources.
"""
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import tempfile
import time
from playwright.sync_api import sync_playwright, expect

ROOT = Path(__file__).resolve().parents[1]
META = {'io.modelcontextprotocol/protocolVersion':'2026-07-28', 'io.modelcontextprotocol/clientCapabilities':{}}
with tempfile.TemporaryDirectory(prefix='vault-compose-') as temporary:
    with socket.socket() as address:
        address.bind(('127.0.0.1', 0))
        port = address.getsockname()[1]
    base = f'http://127.0.0.1:{port}'
    project = 'vault-consolidation-'+secrets.token_hex(4)
    token = secrets.token_hex(32)
    env = {k:v for k,v in os.environ.items() if not k.startswith(('KV_','ACCESS_'))}
    env.update(KV_PORT=str(port), ACCESS_PUBLIC_URL=base, KV_PUBLIC_URL=base, ACCESS_SETUP_TOKEN=token)
    override = Path(temporary)/'override.yml'
    override.write_text(f'''services:
  keyword-vault:
    image: keyword-vault:consolidation-runtime
    container_name: {project}-api
  nginx:
    image: keyword-vault:consolidation-frontend
    container_name: {project}-nginx
    ports: !override
      - "127.0.0.1:{port}:80"
''')
    command = ['docker','compose','--env-file','/dev/null','-p',project,'-f',str(ROOT/'docker-compose.yml'),'-f',str(override)]
    def compose(*args):
        result = subprocess.run(command+list(args), env=env, cwd=ROOT, capture_output=True, text=True)
        if result.returncode:
            raise RuntimeError((args, result.stdout+result.stderr))
        return result.stdout
    try:
        config = json.loads(compose('config','--format','json'))
        assert not config['services']['keyword-vault'].get('ports')
        assert config['services']['keyword-vault']['environment']['ACCESS_PUBLIC_URL'] == base
        compose('up','-d','--no-build')
        compose('exec','-T','nginx','nginx','-t')
        with sync_playwright() as p:
            browser = p.chromium.launch(channel='chrome', headless=True)
            context = browser.new_context(viewport={'width':1440,'height':1000}, reduced_motion='reduce')
            request = context.request
            anonymous = browser.new_context().request
            def call(method, path, status, body=None, headers=None, client=None):
                response = (client or request).fetch(base+path, method=method, data=body, headers={'Origin':base, **(headers or {})})
                assert response.status == status, (method,path,response.status,response.text())
                if response.status == 204:
                    return None
                try:
                    return response.json()
                except Exception:
                    for line in response.text().splitlines():
                        if line.startswith('data: '):
                            return json.loads(line[6:])
                    return response.text()
            for _ in range(120):
                try:
                    if request.get(base+'/api/health').status == 200:
                        break
                except Exception:
                    pass
                time.sleep(.5)
            assert request.get(base+'/healthz').ok
            assert request.get(base+'/assets/missing.js').status == 404
            state = call('GET','/api/auth/state',200)
            assert state['setup_required'] and state['setup_token_required'] and state['setup_token_configured']
            page = context.new_page()
            errors = []
            page.on('pageerror', lambda error: errors.append(str(error)))
            for width, layout in [(1440,'desktop'),(390,'mobile')]:
                for theme in ['light','dark']:
                    page.set_viewport_size({'width':width,'height':1000 if width==1440 else 844})
                    page.goto(base+'/first-login',wait_until='networkidle')
                    page.evaluate('(theme)=>localStorage.setItem("kv.theme",theme)',theme)
                    page.reload(wait_until='networkidle')
                    expect(page.get_by_label('部署設定代碼',exact=True)).to_be_visible()
                    page.evaluate('document.fonts.ready')
                    assert page.evaluate('document.documentElement.scrollWidth <= innerWidth')
                    page.screenshot(path=str(ROOT/f'docs/images/consolidation-first-login-{layout}-{theme}.png'),full_page=True)
            account = {'username':'qa.admin','password':'Disposable Compose password 2026'}
            call('POST','/api/auth/setup',403,account,{'X-Forwarded-For':'127.0.0.1'})
            call('POST','/api/auth/setup',403,{**account,'setup_token':'wrong'})
            call('POST','/api/auth/setup',200,{**account,'setup_token':token})
            call('POST','/api/auth/setup',409,{**account,'setup_token':token})
            record = call('POST','/api/keywords',200,{'theme':'preserve','expression':'珍珠奶茶|台灣高鐵','visibility':'private'})
            call('POST','/api/auth/register',403,{'username':'closed.user','password':account['password']},client=anonymous)
            call('PATCH','/api/auth/registration-policy',200,{'mode':'approval'})
            pending = call('POST','/api/auth/register',201,{'username':'pending.user','password':account['password']},client=anonymous)
            assert pending['status'] == 'pending'
            call('POST','/api/auth/login',403,{'username':'pending.user','password':account['password']},client=anonymous)
            credentials = call('POST','/api/access/credentials',201,{'name':'Consolidation verification','scopes':['queries:read','expressions:read'],'audiences':['api','mcp']})
            bearer = {'Authorization':'Bearer '+credentials['token']}
            operations = [('compile_query',{'expression':'茶|奶'}),('inspect_query',{'expression':'茶!廣告'}),('extract_terms',{'expression':'珍珠奶茶'}),('inspect_workbench',{'expression':'茶|奶','filter_expression':'臺灣|台灣','default_slop':8}),('analyze_expression',{'expression':'珍珠奶茶!廣告','mode':'search'})]
            for name, arguments in operations:
                call('POST','/api/v1/'+name,200,arguments,bearer)
            for number, (name, arguments) in enumerate(operations,1):
                result = call('POST','/mcp',200,{'jsonrpc':'2.0','id':number,'method':'tools/call','params':{'name':name,'arguments':arguments,'_meta':META}}, {**bearer,'Accept':'application/json, text/event-stream','MCP-Protocol-Version':'2026-07-28','Mcp-Method':'tools/call','Mcp-Name':name})
                assert 'result' in result and not result['result'].get('isError'),result
            for transport in ['api','mcp']:
                usage = call('GET','/api/access/usage?transport='+transport,200)
                assert usage['totals']['attempts'] == 5 and usage['totals']['successes'] == 5,usage
            for number, name in enumerate(['compile_query','inspect_query','extract_terms'],10):
                expected = {'code':'compile_error','error':'consecutive operators are not allowed (at character 2)'}
                assert call('POST','/api/v1/'+name,400,{'expression':'a&&b'},bearer) == expected
                failed = call('POST','/mcp',200,{'jsonrpc':'2.0','id':number,'method':'tools/call','params':{'name':name,'arguments':{'expression':'a&&b'},'_meta':META}}, {**bearer,'Accept':'application/json, text/event-stream','MCP-Protocol-Version':'2026-07-28','Mcp-Method':'tools/call','Mcp-Name':name})
                assert failed['result']['isError'] is True and failed['result']['structuredContent'] == expected,failed
            for transport in ['api','mcp']:
                usage = call('GET','/api/access/usage?transport='+transport,200)
                assert usage['totals']['attempts'] == 8 and usage['totals']['failures'] == 3,usage
            before_api = compose('ps','-q','keyword-vault').strip()
            before_front = compose('ps','-q','nginx').strip()
            inspection = json.loads(subprocess.run(['docker','inspect',before_api],capture_output=True,text=True,check=True).stdout)[0]
            assert inspection['Config']['User'] == 'app'
            assert not any(inspection['NetworkSettings']['Ports'].values())
            compose('up','-d','--no-deps','--force-recreate','--no-build','keyword-vault')
            for _ in range(120):
                try:
                    if request.get(base+'/api/auth/state').json().get('user',{}).get('username') == 'qa.admin':
                        break
                except Exception:
                    pass
                time.sleep(.5)
            assert call('GET','/api/keywords/'+str(record['id']),200)['expression'] == record['expression']
            assert compose('ps','-q','nginx').strip() == before_front
            assert compose('ps','-q','keyword-vault').strip() != before_api
            assert call('GET','/api/access/usage?transport=mcp',200)['totals']['successes'] == 5
            call('DELETE','/api/access/credentials/'+credentials['credential']['id'],204)
            call('POST','/api/v1/compile_query',401,{'expression':'茶'},bearer)
            logs = compose('logs','--no-color')
            assert all(secret not in logs for secret in [token,account['password'],credentials['token']])
            assert not errors,errors
            browser.close()
        print(json.dumps({'result':'passed','checks':['nginx configuration/health/static404','real remote setup token and forwarded spoof denial','initial setup screenshots desktop/mobile light/dark','registration pending restriction','five compiler/analysis API + MCP operations, exactly ten successful attempts','legacy parser errors retain compile_error and English text over HTTP/MCP, exactly six failed attempts','nonroot internal API','backend recreate preserves cookie, keyword and usage; nginx stable','revoked credentials rejected','logs omit credentials']},ensure_ascii=False))
    finally:
        compose('down','--volumes','--remove-orphans')
