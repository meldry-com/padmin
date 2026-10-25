const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn,spawnSync}=require('node:child_process'),crypto=require('node:crypto');
const root=process.env.PADMIN_ACCEPTANCE_SOURCE_ROOT?path.resolve(process.env.PADMIN_ACCEPTANCE_SOURCE_ROOT):path.resolve(__dirname,'..'),cache='D:/BuildCache/padmin-glory-branch';
const real=process.argv.includes('--real');
const runtime=fs.existsSync(cache+'/runtime/node_modules/playwright')?cache+'/runtime':'D:/BuildCache/padmin-glory/runtime';
const env={...process.env,TEMP:cache+'/tmp',TMP:cache+'/tmp',PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD:'1',PADMIN_PLAYWRIGHT:process.env.PADMIN_PLAYWRIGHT||runtime+'/node_modules/playwright'};
fs.mkdirSync(env.TEMP,{recursive:true});
const suites=real?['real-backend-acceptance.cjs','browser-real-matrix.cjs','browser-real-domains.cjs','browser-real-operations.cjs','browser-real-stack.cjs','browser-real-source.cjs']:['browser-fixture.cjs','browser-auth.cjs','browser-shell.cjs','browser-source-behaviors.cjs','browser-domains.cjs','browser-domains-writes.cjs','browser-rooms-writes.cjs','browser-operations.cjs'];
const base=real?(env.PADMIN_REAL_URL||'http://127.0.0.1:28789'):'http://127.0.0.1:8789';
function run(file){return new Promise(resolve=>{const child=spawn(process.execPath,[path.join(__dirname,file)],{cwd:root,env,stdio:'inherit',windowsHide:true});child.once('error',e=>resolve({exit_code:-1,error:e.message}));child.once('exit',code=>resolve({exit_code:code??-1}));});}
const evidence={
 'browser-fixture.cjs':'browser-results.json','browser-auth.cjs':'browser-auth-results.json','browser-shell.cjs':'browser-shell-results.json',
 'browser-source-behaviors.cjs':'source-behaviors-results.json','browser-domains.cjs':'browser-domains-results.json',
 'browser-domains-writes.cjs':'browser-domains-writes-results.json','browser-rooms-writes.cjs':'browser-rooms-writes-results.json','browser-operations.cjs':'operations-results.json',
 'real-backend-acceptance.cjs':'real-backend-results.json','browser-real-matrix.cjs':'browser-real-matrix-results.json','browser-real-domains.cjs':'real-pasion-domain-results.json',
 'browser-real-source.cjs':'real-source-results.json','browser-real-operations.cjs':'real-operation-results.json','browser-real-stack.cjs':'browser-real-stack-results.json',
};
const delay=ms=>new Promise(r=>setTimeout(r,ms));
(async()=>{
 assert.ok(fs.existsSync(env.PADMIN_PLAYWRIGHT),'Install Playwright under D:/BuildCache/padmin-glory/runtime, or set PADMIN_PLAYWRIGHT');
 const wasmPath=cache+'/dist/pkg/padmin_bg.wasm';assert.ok(fs.existsSync(wasmPath),'Build actual WASM with python scripts/glory.py build --dev before acceptance');
 const info=JSON.parse(fs.readFileSync(cache+'/dist/build-info.json','utf8'));
 const fingerprint=spawnSync('python',[path.join(root,'scripts/glory.py'),'fingerprint'],{cwd:root,env,encoding:'utf8',windowsHide:true});
 assert.equal(fingerprint.status,0,fingerprint.stderr);
 const current=JSON.parse(fingerprint.stdout.trim().split(/\r?\n/).at(-1));
 assert.deepEqual(current.source_files,info.source_files,'App/Glory/Sapid input membership changed; rebuild before acceptance');
 assert.equal(current.source_sha256,info.source_sha256,'App/Glory/Sapid sources differ from the compiled build; rebuild before acceptance');
 const localWasm=crypto.createHash('sha256').update(fs.readFileSync(wasmPath)).digest('hex');assert.equal(localWasm,info.wasm_sha256,'Build info does not match local WASM');
 let server;try{
  try{await fetch(base+'/healthz',{signal:AbortSignal.timeout(1000)});}catch(e){if(real)throw Error('Start the isolated real stack and node scripts/real-server.cjs before test:real');server=spawn(process.execPath,[path.join(__dirname,'real-server.cjs')],{cwd:root,env:{...env,PORT:'8789'},stdio:'ignore',windowsHide:true});}
  let bytes;for(let i=0;i<50;i++){try{const r=await fetch(base+'/pkg/padmin_bg.wasm',{signal:AbortSignal.timeout(2000)});assert.equal(r.status,200);bytes=Buffer.from(await r.arrayBuffer());assert.ok(bytes.length>10000);assert.equal(bytes.subarray(0,4).toString('hex'),'0061736d');break;}catch(e){if(i===49)throw e;await delay(100);}}
  const served=crypto.createHash('sha256').update(bytes).digest('hex');assert.equal(served,info.wasm_sha256,'Server serves a different WASM build');
  const results=[];for(const suite of suites){console.log('\nRunning '+suite);const start=Date.now(),result=await run(suite);const artifact=cache+'/'+evidence[suite];if(result.exit_code===0){assert.ok(fs.existsSync(artifact),suite+' omitted evidence');assert.ok(fs.statSync(artifact).mtimeMs>=start-1000,suite+' retained stale evidence');}results.push({suite,...result,evidence:artifact,started_at:new Date(start).toISOString()});}
  const report={base,source_root:root,test_root:__dirname,wasm_sha256:served,source_sha256:info.source_sha256,suites:results};
  fs.writeFileSync(cache+(real?'/acceptance-real.json':'/acceptance-fixtures.json'),JSON.stringify(report,null,2));
  let combined={};try{combined=JSON.parse(fs.readFileSync(cache+'/acceptance-results.json','utf8'));}catch{}
  if(combined.wasm_sha256!==served)combined={wasm_sha256:served,source_sha256:info.source_sha256};combined[real?'real':'fixtures']=report;
  fs.writeFileSync(cache+'/acceptance-results.json',JSON.stringify(combined,null,2));
  assert.ok(results.every(r=>r.exit_code===0),'Acceptance suites failed; see '+cache+'/acceptance-results.json');
 }finally{if(server)server.kill();}
})().catch(e=>{console.error(e);process.exitCode=1;});
