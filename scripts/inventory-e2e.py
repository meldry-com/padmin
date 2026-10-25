"""Inventory every original test/assertion without weakening backend assertions."""
from pathlib import Path
import json,re
root=Path(__file__).resolve().parents[1]
rows=[]
for source in sorted((root/'e2e').glob('*.spec.ts')):
    lines=source.read_text(encoding='utf-8').splitlines()
    tests=[]
    for number,line in enumerate(lines,1):
        m=re.search(r'\btest\("([^"]+)"',line)
        if m:tests.append((number,m.group(1)))
    suite=('scripts/browser-domains.cjs' if source.stem.split('.')[0] in ['users','rooms'] else 'scripts/browser-operations.cjs' if source.stem.split('.')[0] in ['appservices','media','reports','federation','registration-tokens'] else 'scripts/browser-shell.cjs')
    backend=source.stem.split('.')[0] in ['compose-full-flow','oidc-flow','pasion-registration','single-origin']
    for index,(start,title) in enumerate(tests):
        end=tests[index+1][0]-1 if index+1<len(tests) else len(lines)
        assertions=[{'line':i,'source':lines[i-1].strip()} for i in range(start,end+1) if re.search(r'\bexpect\(|\.to(Be|Have|Contain|Equal|Match|Throw)|\bassert\.',lines[i-1])]
        rows.append({'suite':source.name,'line':start,'test':title,'assertions':assertions,'target':str(source.relative_to(root)) if backend else suite,'backend_required':backend,'status':'preserved; execution evidence required'})
output=root/'docs/e2e-behavior-map.json'
output.write_text(json.dumps({'original_suites':18,'inventoried_suites':len(list((root/'e2e').glob('*.spec.ts'))),'test_count':len(rows),'assertion_line_count':sum(len(r['assertions']) for r in rows),'tests':rows},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f'{len(rows)} tests, {sum(len(r["assertions"]) for r in rows)} assertion lines; {output}')
