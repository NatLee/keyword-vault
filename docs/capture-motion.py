"""Record actual state changes; verify final reduced-motion and responsive states.
Uses only an isolated local demo. No credential or session is written to the repo.
"""
import json, os, shutil, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright, expect
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'docs/images'
CHECK_STYLE="""() => {
 const issues=[];
 for(const el of document.querySelectorAll('body *')) {
  if(!el.getClientRects().length || el.closest('svg')) continue;
  const s=getComputedStyle(el);
  const props=['paddingTop','paddingRight','paddingBottom','paddingLeft','marginTop','marginRight','marginBottom','marginLeft','rowGap','columnGap'];
  for(const p of props) {
   const authored=el.computedStyleMap?.().get(p.replace(/[A-Z]/g,m=>'-'+m.toLowerCase()))?.toString();
   if(authored==='auto') continue;
   const n=parseFloat(s[p]);
   if(s[p].endsWith('px') && Math.abs(n/4-Math.round(n/4))>.01) issues.push(`${el.tagName}.${el.className}: ${p}=${s[p]}`);
  }
  if(!['400','600'].includes(s.fontWeight)) issues.push(`${el.tagName}.${el.className}: weight=${s.fontWeight}`);
 }
 return [...new Set(issues)];
}"""
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True,channel=os.environ.get('PLAYWRIGHT_CHANNEL','chrome'))
 ERR=[]
 URL=os.environ.get('KV_QA_URL','http://127.0.0.1:18686')
 login=browser.new_context()
 tab=login.new_page();tab.goto(URL)
 tab.get_by_label('帳號',exact=True).fill(os.environ.get('KV_QA_USER','demo'))
 tab.get_by_label('密碼',exact=True).fill(os.environ['KV_QA_PASSWORD'])
 tab.get_by_role('button',name='登入',exact=True).click()
 expect(tab.get_by_role('heading',name='語法工作台')).to_be_visible()
 tab.get_by_role('radio',name='淺色',exact=True).click()
 state={'storage_state':login.storage_state()}
 login.close()
 URL+='/inspect?keyword=1'
 with tempfile.TemporaryDirectory() as videos:
  ctx=browser.new_context(viewport={'width':1440,'height':1000},record_video_dir=videos,record_video_size={'width':1440,'height':1000},reduced_motion='no-preference',**state)
  page=ctx.new_page(); page.on('pageerror',lambda e:ERR.append(str(e)))
  page.goto(URL,wait_until='networkidle')
  expect(page.get_by_label('草稿語法')).not_to_have_value('')
  page.evaluate("window.scrollTo(0,document.querySelector('.inspection-workspace').getBoundingClientRect().top+scrollY-16)")
  for number in [2,3,4,5,1]:
   node=page.locator('.anatomy-node').nth(number-1)
   node.focus();page.keyboard.press('Enter');page.wait_for_timeout(700)
   expect(node).to_have_attribute('aria-pressed','true')
  assert page.get_by_label('草稿語法').input_value()!=''
  # Inspect authored spacing together with browser-computed inherited/default styles.
  styles=page.evaluate(CHECK_STYLE)
  assert not styles, styles[:30]
  for width in [1440,390]:
   page.set_viewport_size({'width':width,'height':1000 if width==1440 else 844})
   assert page.evaluate('document.documentElement.scrollWidth <= innerWidth'), f'overflow {width}'
  page.emulate_media(reduced_motion='reduce')
  node=page.locator('.anatomy-node').nth(1)
  node.focus();page.keyboard.press('Enter')
  expect(node).to_have_attribute('aria-pressed','true')
  expect(node).to_be_focused()
  assert page.locator('.anatomy-note').evaluate('el=>getComputedStyle(el).animationName')=='none'
  assert not ERR, ERR
  video=page.video
  ctx.close()
  shutil.copyfile(video.path(),OUT/'feature-motion.webm')
 audit=browser.new_context(viewport={'width':1440,'height':1000},reduced_motion='reduce',**state)
 check=audit.new_page()
 for theme in ['light','dark']:
  for width in [1440,390]:
   check.set_viewport_size({'width':width,'height':1000})
   for route in ['/','/keywords','/keywords/1','/compare','/guide','/settings','/inspect?keyword=1']:
    check.goto(os.environ.get('KV_QA_URL','http://127.0.0.1:18686')+route,wait_until='networkidle')
    check.evaluate('(theme)=>document.documentElement.classList.toggle("dark",theme==="dark")',theme)
    assert not check.evaluate(CHECK_STYLE), (theme,width,route,check.evaluate(CHECK_STYLE)[:15])
    assert check.evaluate('document.documentElement.scrollWidth<=innerWidth'), (theme,width,route)
 audit.close()
 browser.close()
 print(json.dumps({'video':'feature-motion.webm','styles':'computed 4px spacing and400/600 passed','widths':[1440,390],'reduced_motion':'final selection visible','page_errors':ERR}))
