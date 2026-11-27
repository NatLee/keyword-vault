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
 state={}
 URL=os.environ.get('ES_QA_URL','http://127.0.0.1:15379/docs/index.html')
 with tempfile.TemporaryDirectory() as videos:
  ctx=browser.new_context(viewport={'width':1440,'height':1000},record_video_dir=videos,record_video_size={'width':1440,'height':1000},reduced_motion='no-preference',**state)
  page=ctx.new_page(); page.on('pageerror',lambda e:ERR.append(str(e)))
  page.goto(URL,wait_until='networkidle')
  page.evaluate("localStorage.setItem('atlas-theme','light')"); page.reload(wait_until='networkidle')
  page.evaluate("window.scrollTo(0,document.querySelector('.atlas').getBoundingClientRect().top+scrollY-16)")
  page.wait_for_timeout(500)
  page.get_by_role('tab',name='00 Source').click(); page.wait_for_timeout(700)
  page.get_by_role('tab',name='01 Normalize').click(); page.wait_for_timeout(700)
  page.get_by_role('tab',name='02 Structure').click(); page.wait_for_timeout(700)
  for label in ['Clause 2: tea','Clause 3: coffee','Clause 5: ad excluded']:
   page.get_by_role('button',name=label,exact=True).click(); page.wait_for_timeout(700)
  expect(page.locator('.clause-label')).to_have_text('ad')
  page.get_by_role('tab',name='03 Compile').click(); page.wait_for_timeout(700)
  expect(page.locator('#stage-panel pre')).to_have_text(page.locator('#compiled').inner_text())
  # Inspect authored spacing together with browser-computed inherited/default styles.
  styles=page.evaluate(CHECK_STYLE)
  assert not styles, styles[:30]
  for width in [1440,390]:
   page.set_viewport_size({'width':width,'height':1000 if width==1440 else 844})
   assert page.evaluate('document.documentElement.scrollWidth <= innerWidth'), f'overflow {width}'
  page.emulate_media(reduced_motion='reduce')
  page.get_by_role('tab',name='02 Structure').click()
  page.get_by_role('button',name='Clause 2: tea',exact=True).click()
  expect(page.locator('.clause-label')).to_have_text('tea')
  assert page.locator('#clause-notes .clause-label').evaluate('el=>getComputedStyle(el).animationName')=='none'
  assert not ERR, ERR
  video=page.video
  ctx.close()
  shutil.copyfile(video.path(),OUT/'feature-motion.webm')
 audit=browser.new_context(viewport={'width':1440,'height':1000},reduced_motion='reduce')
 check=audit.new_page();check.goto(URL)
 for theme in ['light','dark']:
  check.evaluate('(theme)=>document.documentElement.classList.toggle("light",theme==="light")',theme)
  for width in [1440,390]:
   check.set_viewport_size({'width':width,'height':1000})
   for number in range(8):
    check.locator('#specimens button').nth(number).click()
    for stage in ['00 Source','01 Normalize','02 Structure','03 Compile']:
     check.get_by_role('tab',name=stage).click()
     assert not check.evaluate(CHECK_STYLE), (theme,width,number,stage,check.evaluate(CHECK_STYLE)[:15])
     assert check.evaluate('document.documentElement.scrollWidth<=innerWidth')
 audit.close()
 browser.close()
 print(json.dumps({'video':'feature-motion.webm','styles':'computed 4px spacing and400/600 passed','widths':[1440,390],'reduced_motion':'final selection visible','page_errors':ERR}))
