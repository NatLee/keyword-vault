"""Actual local access workflow; never target production or record raw key screenshots.
Build the host with access-kit/test-support and run the loopback mock provider.
ACCESS_CAPTURE_PASSWORD_FILE=/private/demo/password python docs/capture-access.py
ACCESS_CAPTURE_URL and ACCESS_CAPTURE_PROBE may override isolated demo defaults.
"""
import base64, hashlib, json, os, queue, secrets, subprocess, threading
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from urllib.parse import urlencode,urlparse,parse_qs
from playwright.sync_api import sync_playwright,expect
ROOT=Path(__file__).resolve().parents[1]
KIND='es'
ES=KIND=='es'
BASE=os.environ.get('ACCESS_CAPTURE_URL','http://127.0.0.1:'+('18670' if ES else '18686')).rstrip('/')
assert urlparse(BASE).scheme=='http' and urlparse(BASE).hostname in {'127.0.0.1','localhost','::1'}, 'Use isolated loopback only'
PASSWORD_FILE=Path(os.environ.get('ACCESS_CAPTURE_PASSWORD_FILE','/tmp/'+('es' if ES else 'kv')+'-access-review/password'))
PROBE=Path(os.environ.get('ACCESS_CAPTURE_PROBE',ROOT/('server' if ES else 'src/api')/'target/debug/examples/access_probe'))
OUT=ROOT/'docs/images';OUT.mkdir(exist_ok=True)
callbacks=queue.Queue()
class Callback(BaseHTTPRequestHandler):
 def do_GET(self):
  if urlparse(self.path).path!='/callback':
   self.send_response(204);self.end_headers();return
  callbacks.put(parse_qs(urlparse(self.path).query));self.send_response(200);self.send_header('content-type','text/plain');self.end_headers();self.wfile.write(b'Local review client received the authorization response. No codes are displayed.')
 def log_message(self,*_args):pass
server=HTTPServer(('127.0.0.1',0),Callback);threading.Thread(target=server.serve_forever,daemon=True).start()
redirect=f'http://127.0.0.1:{server.server_port}/callback'
operation='compile_query' if ES else 'inspect_expression'
arguments={'expression':'SensitiveAccessCanary'}
scope='queries:read' if ES else 'keywords:read'
errors=[]
def sdk(token,allowed=True):
 env={**os.environ,'ACCESS_TEST_BASE':BASE,'ACCESS_TEST_TOKEN':token,'ACCESS_TEST_OPERATION':operation if allowed else ('extract_terms' if ES else 'search_keywords'),'ACCESS_TEST_ARGUMENTS':json.dumps(arguments if allowed else ({'expression':'tea'} if ES else {}))}
 result=subprocess.run([str(PROBE)],env=env,capture_output=True,text=True,timeout=20)
 assert (result.returncode==0)==allowed, 'Official SDK probe outcome mismatch; no bearer details are printed'

def settle(page):
 page.wait_for_load_state('networkidle');page.evaluate('document.fonts.ready')

def shot(page,number,target):
 assert page.get_by_role('dialog').count()==0 or number==3, 'Close the original API key before any capture'
 target.scroll_into_view_if_needed();settle(page)
 target.screenshot(path=str(OUT/f'access-{number:02}.png'),animations='disabled')

try:
 with sync_playwright() as p:
  browser=p.chromium.launch(channel='chrome');context=browser.new_context(viewport={'width':1440,'height':1080},color_scheme='light',reduced_motion='reduce');page=context.new_page();page.on('pageerror',lambda e:errors.append(str(e)))
  page.goto(BASE,wait_until='networkidle');page.get_by_label('Username' if ES else '帳號',exact=True).fill('demo');page.get_by_label('Password' if ES else '密碼',exact=True).fill(PASSWORD_FILE.read_text());
  with page.expect_response(lambda r:r.url==BASE+'/api/auth/login' and r.request.method=='POST') as signed_in:
   page.get_by_role('button',name='Sign in' if ES else '登入',exact=True).click()
  assert signed_in.value.ok, 'Local demo login failed'
  expect(page.get_by_role('button',name='Sign out' if ES else '登出',exact=True)).to_be_visible(timeout=10000)
  settle(page)
  page.goto(BASE+'/connections',wait_until='networkidle');expect(page.get_by_role('heading',name='Connections and access' if ES else '連線與存取',exact=True)).to_be_visible()
  for provider in ['GitHub','Google']:
   link=page.get_by_role('button',name=('Link ' if ES else '綁定 ')+provider,exact=True)
   if link.count():
    expect(link).to_be_enabled();link.click();expect(page.get_by_role('button',name=('Unlink ' if ES else '解除 ')+provider,exact=True)).to_be_visible();settle(page)
  identities=context.request.get(BASE+'/api/access/identities').json()['items'];assert {i['provider'] for i in identities}=={'github','google'}
  before=context.request.get(BASE+'/api/access/usage').json()['totals']['attempts']
  suffix=secrets.token_hex(3);key_name='Local SDK review '+suffix
  page.get_by_label('Key name' if ES else '金鑰名稱',exact=True).fill(key_name)
  with page.expect_response(lambda r:r.url==BASE+'/api/access/credentials' and r.request.method=='POST') as created:
   page.get_by_role('button',name='Create API key' if ES else '建立 API 金鑰',exact=True).click()
  response=created.value.json();token=response['token'];key_id=response['credential']['id']
  expect(page.get_by_text(token,exact=True)).to_be_visible()
  close=page.get_by_role('button',name='Close key value' if ES else '關閉金鑰原值',exact=True);expect(close).to_be_focused();page.keyboard.press('Escape');expect(page.get_by_text(token,exact=True)).to_have_count(0)
  expect(page.get_by_role('button',name='Create API key' if ES else '建立 API 金鑰',exact=True)).to_be_focused()
  assert not page.evaluate('(token)=>Object.values(localStorage).some(value=>value.includes(token))',token)
  listed=context.request.get(BASE+'/api/access/credentials').json();assert token not in json.dumps(listed)
  headers={'Authorization':'Bearer '+token}
  result=context.request.post(BASE+'/api/v1/'+operation,headers=headers,data=arguments);assert result.ok
  sdk(token)
  page.get_by_role('button',name='Refresh' if ES else '重新整理',exact=True).click();settle(page)
  overview=context.request.get(BASE+'/api/access/overview').json();assert overview['summary']['attempts']==before+2
  key_section=page.locator('section').filter(has=page.get_by_role('heading',name='API keys' if ES else 'API 金鑰',exact=True)).first
  shot(page,1,key_section)
  # Register a real public client using the native admin form.
  client_name='Local research client '+suffix
  page.get_by_label('Application name' if ES else '應用名稱',exact=True).fill(client_name)
  page.get_by_label('Redirect URIs (one HTTPS or loopback URI per line)' if ES else '回呼網址（每行一個 HTTPS 或 loopback URI）',exact=True).fill(redirect)
  with page.expect_response(lambda r:r.url==BASE+'/api/access/clients' and r.request.method=='POST') as registered:
   page.get_by_role('button',name='Register application' if ES else '註冊應用',exact=True).click()
  client_id=registered.value.json()['client_id'];settle(page)
  verifier=secrets.token_urlsafe(48);challenge=base64.urlsafe_b64encode(hashlib.sha256(verifier.encode()).digest()).decode().rstrip('=');state=secrets.token_urlsafe(24)
  params={'response_type':'code','client_id':client_id,'redirect_uri':redirect,'scope':scope,'resource':BASE+'/mcp','state':state,'code_challenge':challenge,'code_challenge_method':'S256'}
  page.goto(BASE+'/oauth/authorize?'+urlencode(params),wait_until='networkidle')
  expect(page.get_by_role('heading',name='Authorize application' if ES else '應用程式授權',exact=True)).to_be_visible();expect(page.get_by_text(client_name,exact=True)).to_be_visible()
  form=page.get_by_role('button',name='Approve access' if ES else '同意授權',exact=True).locator('..')
  assert form.locator('[name="scopes"]').count()==0
  consent=page.locator('section.consent' if ES else '.access-consent');shot(page,3,consent)
  page.get_by_role('button',name='Approve access' if ES else '同意授權',exact=True).click();response=callbacks.get(timeout=10);assert response['state']==[state] and 'code' in response
  redeemed=context.request.post(BASE+'/oauth/token',form={'grant_type':'authorization_code','client_id':client_id,'redirect_uri':redirect,'code':response['code'][0],'code_verifier':verifier,'resource':BASE+'/mcp'});assert redeemed.ok;assert redeemed.headers.get('cache-control')=='no-store';oauth_token=redeemed.json()['access_token']
  # This grant has only keyword scope in Vault; its actual domain operation is search.
  original=operation
  if not ES:operation='search_keywords';arguments={}
  sdk(oauth_token)
  operation=original;arguments={'expression':'SensitiveAccessCanary'}
  page.goto(BASE+'/connections',wait_until='networkidle');usage=context.request.get(BASE+'/api/access/usage').json();assert usage['totals']['attempts']==before+3
  ledger=page.locator('section').filter(has=page.get_by_role('heading',name='Usage and audit' if ES else '用量與存取紀錄',exact=True)).first;shot(page,2,ledger)
  grants=context.request.get(BASE+'/api/access/grants').json()['items'];grant=next(g for g in grants if g['client_id']==client_id)
  page.get_by_role('button',name=('Revoke application ' if ES else '撤銷應用 ')+client_name,exact=True).click();settle(page);sdk(oauth_token,False)
  # A separate immutable consent request exercises the native deny path.
  state=secrets.token_urlsafe(24);params['state']=state
  page.goto(BASE+'/oauth/authorize?'+urlencode(params),wait_until='networkidle');page.get_by_role('button',name='Deny' if ES else '拒絕',exact=True).click();denied=callbacks.get(timeout=10);assert denied['state']==[state] and denied['error']==['access_denied']
  page.goto(BASE+'/connections',wait_until='networkidle');page.get_by_role('button',name=('Revoke ' if ES else '撤銷 ')+key_name,exact=True).click();settle(page)
  assert context.request.post(BASE+'/api/v1/'+operation,headers=headers,data=arguments).status==401;sdk(token,False)
  events=context.request.get(BASE+'/api/access/events?user_id=all').json();serialized=json.dumps(events);assert token not in serialized and oauth_token not in serialized and 'SensitiveAccessCanary' not in serialized
  assert context.request.get(BASE+'/api/access/usage').json()['totals']['attempts']==before+3
  # Local server logs must not contain tool payload or raw bearer values, even at trace.
  log=Path('/tmp/'+('es' if ES else 'kv')+'-access-review/server.log')
  if log.exists():
   text=log.read_text();assert token not in text and oauth_token not in text and 'SensitiveAccessCanary' not in text
  for width in [1440,390]:
   page.set_viewport_size({'width':width,'height':1080 if width==1440 else 844});assert page.evaluate('document.documentElement.scrollWidth<=innerWidth')
  assert not errors,errors
  context.close();browser.close()
  print('PASS: linked local GitHub/Google; native key create/close; real REST+official MCP; native OAuth approve/deny; exact3usage attempts; key/grant revoke immediate; no raw key/payload in audit/logs;1440/390;3actual PNGs')
finally:server.shutdown();server.server_close()
