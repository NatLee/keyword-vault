"""Browser regression for ledger request races; only mocked audit GETs, no domain writes.
Run with a local authenticated demo and ACCESS_LEDGER_PASSWORD_FILE; Playwright Chrome.
"""
import argparse,json,os
from pathlib import Path
from urllib.parse import urlparse,parse_qs
from playwright.sync_api import sync_playwright,expect
KIND="es"
ES=KIND=="es"
parser=argparse.ArgumentParser();parser.add_argument('--case',choices=['reload','pagination','draft','duplicate','all'],default='all');args=parser.parse_args()
BASE=os.environ.get('ACCESS_LEDGER_URL','http://127.0.0.1:'+('18670' if ES else '18686')).rstrip('/')
assert urlparse(BASE).hostname in {'127.0.0.1','localhost'},'Isolated loopback demo only'
password=Path(os.environ.get('ACCESS_LEDGER_PASSWORD_FILE','/tmp/'+KIND+'-access-review/password')).read_text()
zero=dict(attempts=0,successes=0,failures=0,pending=0)
def payload(query,cursor=False):
 owner=query.get('user_id',['self'])[0];transport=query.get('transport',['all'])[0];name=owner+'-'+transport+('-older' if cursor else '-first')
 event=dict(id=name,at='2026-11-27T00:00:00Z',transport=transport,operation='compile_query',outcome='success',status=200,request_id=name,duration_ms=1)
 return dict(items=[event],next_cursor=None if cursor else 'cursor-'+owner+'-'+transport)
def respond(route,data):route.fulfill(status=200,content_type='application/json',body=json.dumps(data))
with sync_playwright() as p:
 browser=p.chromium.launch(channel='chrome');ctx=browser.new_context();page=ctx.new_page();page.goto(BASE+'/login')
 page.get_by_label('Username' if ES else '帳號',exact=True).fill('demo');page.get_by_label('Password' if ES else '密碼',exact=True).fill(password)
 page.get_by_role('button',name='Sign in' if ES else '登入',exact=True).click();expect(page.get_by_role('button',name='Sign out' if ES else '登出',exact=True)).to_be_visible();state=ctx.storage_state();ctx.close()
 cases=['reload','pagination','draft','duplicate'] if args.case=='all' else [args.case]
 for case in cases:
  ctx=browser.new_context(storage_state=state);page=ctx.new_page();held=[];calls=[]
  def intercept(route):
   url=urlparse(route.request.url);q=parse_qs(url.query)
   if url.path.endswith('/usage'):
    respond(route,dict(days=int(q.get('days',['7'])[0]),**{'from':'2026-11-21T00:00:00Z','to':'2026-11-28T00:00:00Z'},totals=zero,daily=[dict(date='2026-11-27',**zero)]));return
   calls.append(q)
   if 'cursor' in q or (case=='reload' and q.get('user_id')==['A']):held.append((route,q));return
   respond(route,payload(q))
  page.route('**/api/access/usage?*',intercept);page.route('**/api/access/events?*',intercept);page.goto(BASE+'/connections');expect(page.get_by_role('cell',name='self-all-first',exact=True)).to_be_visible()
  owner=page.get_by_label('User ID (blank: me, all: everyone)' if ES else '使用者編號（空白為本人／all 為全體）',exact=ES)
  transport=page.get_by_label('Transport' if ES else '通道',exact=ES)
  more=page.get_by_role('button',name='Load earlier events' if ES else '載入較早紀錄',exact=True)
  def apply_owner(value,channel):
   owner.fill(value)
   if ES:
    transport.select_option(channel);page.get_by_role('button',name='Apply filters',exact=True).click()
   else:
    transport.select_option(channel);page.get_by_role('button',name='套用使用者',exact=True).click()
  if case=='reload':
   apply_owner('A','api');page.wait_for_timeout(80);assert held,'A request must actually be held'
   apply_owner('B','mcp');expect(page.get_by_role('cell',name='B-mcp-first',exact=True)).to_be_visible()
   for route,q in held:respond(route,payload(q))
   page.wait_for_timeout(100);expect(page.get_by_role('cell',name='B-mcp-first',exact=True)).to_be_visible();expect(page.get_by_role('cell',name='A-api-first',exact=True)).to_have_count(0)
  elif case=='pagination':
   more.click();page.wait_for_timeout(80);assert held,'Old pagination must actually be held'
   apply_owner('B','mcp');expect(page.get_by_role('cell',name='B-mcp-first',exact=True)).to_be_visible()
   for route,q in held:respond(route,payload(q,True))
   page.wait_for_timeout(100);expect(page.get_by_role('cell',name='self-all-older',exact=True)).to_have_count(0);expect(more).to_be_enabled()
   more.click();page.wait_for_timeout(80);q=calls[-1];assert q.get('cursor')==['cursor-B-mcp'] and q.get('user_id')==['B'] and q.get('transport')==['mcp'],q
  elif case=='draft':
   owner.fill('UNAPPLIED')
   if ES:transport.select_option('api')
   more.click();page.wait_for_timeout(80);q=calls[-1];assert q.get('cursor')==['cursor-self-all'] and 'user_id' not in q and 'transport' not in q,q
  else:
   more.evaluate('(b)=>{b.click();b.click()}');page.wait_for_timeout(80);assert len(held)==1,('duplicate pagination requests',len(held))
   expect(more).to_be_disabled();overlap=payload(held[0][1],True);overlap['items']=[payload(held[0][1])['items'][0],*overlap['items'],*overlap['items']];respond(held[0][0],overlap);page.wait_for_timeout(80);expect(page.get_by_role('cell',name='self-all-older',exact=True)).to_have_count(1);expect(page.get_by_role('cell',name='self-all-first',exact=True)).to_have_count(1)
  print('PASS',KIND,case,flush=True)
  for route,q in held:
   try:respond(route,payload(q,'cursor' in q))
   except Exception:pass
  ctx.close()
 browser.close()
