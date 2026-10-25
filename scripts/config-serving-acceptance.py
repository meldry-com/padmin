"""Actual clean static output/config HTTP acceptance without another Rust compilation."""
import importlib.util,json,os,subprocess,sys,time,urllib.request,hashlib,shutil
from pathlib import Path
sys.dont_write_bytecode=True
root=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('glory_build',root/'scripts/glory.py');glory=importlib.util.module_from_spec(spec);spec.loader.exec_module(glory)
output=Path('D:/BuildCache/padmin-glory-branch/clean-serving');output.mkdir(exist_ok=True)
config=output/'config.json'
if config.exists():config.unlink()
assert not config.exists()
glory.write_public_config(output)
assert json.loads(config.read_text())=={'pasion_public_url':'','oauth_client_id':'01KMQPADM1N000000000000000','palpo_admin_url':''}
# Validate full JSON encoding before testing normal empty same-origin configuration.
expected={'pasion_public_url':'https://example.test/a?x="quoted"&unicode=中文','oauth_client_id':'client\\quoted"','palpo_admin_url':'/_sidecar'}
glory.write_public_config(output,**expected)
assert json.loads(config.read_text(encoding='utf-8'))==expected
for name in ['bootstrap.js','style.css','pasion-config.js']:
 shutil.copyfile(Path('D:/BuildCache/padmin-glory-branch/dist')/name,output/name)
(output/'index.html').write_text((root/'index.html').read_text().replace('<base href="/">','<base href="/admin/">'))
(output/'runtime-config.js').write_text('window.PADMIN_BASE_PATH="/admin";window.PADMIN_PASION_BASE_PATH="";')
if not (output/'pkg').exists():
 subprocess.run(['powershell','-NoProfile','-Command',"New-Item -ItemType Junction -Path 'D:/BuildCache/padmin-glory-branch/clean-serving/pkg' -Target 'D:/BuildCache/padmin-glory-branch/dist/pkg' | Out-Null"],check=True)
code="import importlib.util,sys;sys.dont_write_bytecode=True;from pathlib import Path;s=importlib.util.spec_from_file_location('glory_build',r'"+str(root/'scripts/glory.py')+"');m=importlib.util.module_from_spec(s);s.loader.exec_module(m);m.serve(Path(r'"+str(output)+"'),'/admin',28990)"
log=open('D:/BuildCache/padmin-glory-branch/clean-serving.log','w')
process=subprocess.Popen([sys.executable,'-B','-c',code],cwd=root,stdout=log,stderr=log,creationflags=subprocess.CREATE_NO_WINDOW,env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'})
try:
 for _ in range(50):
  try:
   with urllib.request.urlopen('http://127.0.0.1:28990/admin/config.json',timeout=1) as response:
    assert response.status==200;assert json.load(response)==expected;break
  except OSError:time.sleep(.1)
 else:raise RuntimeError('Clean static host did not become reachable')
 glory.write_public_config(output)
 browser_script="""const {chromium}=require('D:/BuildCache/padmin-glory/runtime/node_modules/playwright');(async()=>{const b=await chromium.launch({channel:'msedge',headless:true});try{const p=await b.newPage();const errors=[];p.on('pageerror',e=>errors.push(e.message));await p.goto('http://127.0.0.1:28990/admin/login');await p.getByRole('heading',{name:'Sign in to Padmin',exact:true}).waitFor();if(errors.length)throw Error(errors.join('\\n'));}finally{await b.close();}})().catch(e=>{console.error(e);process.exitCode=1});"""
 subprocess.run(['node','-e',browser_script],cwd=root,check=True)
 result={'passed':['Fresh output creates valid same-origin public config before serving','Explicit public URL/client/sidecar round-trip quotes, backslashes and Unicode through JSON','Actual static /admin/config.json returns HTTP200 with exactly the generated public values'], 'wasm_sha256':hashlib.sha256(Path('D:/BuildCache/padmin-glory-branch/dist/pkg/padmin_bg.wasm').read_bytes()).hexdigest(),'scope':'Host packaging configuration only; compiled WASM unchanged and no backend proxy is claimed'}
 info=json.loads(Path('D:/BuildCache/padmin-glory-branch/dist/build-info.json').read_text());assert result['wasm_sha256']==info['wasm_sha256'];result['source_sha256']=info['source_sha256'];result['exit_code']=0
 result['passed'].append('Clean static output boots the actual release WASM and shows Sign in under /admin without network fixtures or configuration repair')
 Path('D:/BuildCache/padmin-glory-branch/clean-serving-results.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
 # Refresh the active dist with ordinary defaults using the identical packaging function.
 glory.write_public_config(Path('D:/BuildCache/padmin-glory-branch/dist'))
 print('\n'.join(result['passed']))
finally:
 process.terminate();process.wait(timeout=5);log.close()
