"""Audit authored spacing/weight/effect rules (not SVG/chart coordinates)."""
from pathlib import Path
import re
ROOT=Path(__file__).resolve().parents[1]
SOURCES=[ROOT/'docs/atlas.css', ROOT/'docs/index.html', ROOT/'docs/images/src/cover.html', *list((ROOT/'server/web').glob('*'))]
issues=[]
scanned=0
for p in SOURCES:
 if not p.is_file() or '.test.' in p.name or p.suffix not in {'.css','.tsx','.html','.mjs'}: continue
 scanned+=1
 s=p.read_text()
 def record(m,reason): issues.append(f"{p.relative_to(ROOT)}:{s[:m.start()].count(chr(10))+1}: {reason}: {m[0]}")
 for m in re.finditer(r'(?:padding|margin)(?:-[a-z-]+)?\s*:\s*([^;{}]+);|(?:row-|column-)?gap\s*:\s*([^;{}]+);',s):
  v=m[1] or m[2]
  if re.search(r'[%]|[0-9](?:vw|vh|rem|em)',v): record(m,'non-fixed spacing')
  for n in re.findall(r'(-?[\d.]+)px',v):
   if float(n)%4: record(m,'non-4px spacing')
 for m in re.finditer(r'font-weight:\s*([0-9]+)',s):
  if m[1] not in {'400','600'}: record(m,'weight')
 for m in re.finditer(r'(?<![\w])(?:p[trblxyse]?|m[trblxyse]?|gap(?:-[xy])?|space-[xy])-[0-9]+\.[0-9]+|font-(?:medium|bold|extrabold|black)|backdrop-blur|filter:\s*blur|box-shadow:(?!\s*none\b)[^;{}]+;',s): record(m,'utility/effect')
print('\n'.join(issues) if issues else f'PASS: spacing / weight / glass / glow source scan ({scanned} files)')
raise SystemExit(bool(issues))
