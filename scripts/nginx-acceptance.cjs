// Tests the actual production entrypoint in a cached nginx container, without builds.
const assert=require('node:assert/strict'),fs=require('node:fs'),crypto=require('node:crypto'),{spawnSync}=require('node:child_process');
const container=process.env.PADMIN_NGINX_CONTAINER||'padmin-glory-nginx';const base=process.env.PADMIN_NGINX_URL||'http://127.0.0.1:28880';
function docker(args){const p=spawnSync('docker',args,{encoding:'utf8'});if(p.status!==0)throw Error(p.stderr);}
const pause=ms=>new Promise(r=>setTimeout(r,ms));
(async()=>{const info=JSON.parse(fs.readFileSync('D:/BuildCache/padmin-glory-branch/dist/build-info.json','utf8'));const wasm=crypto.createHash('sha256').update(fs.readFileSync('D:/BuildCache/padmin-glory-branch/dist/pkg/padmin_bg.wasm')).digest('hex');assert.equal(wasm,info.wasm_sha256);const results=[];
for(const prefix of ['', '/admin','/console/manage']){
 docker(['exec','--user','nginx',container,'nginx','-s','quit']);await pause(100);
 docker(['exec','-d','--user','nginx','-e','PADMIN_BASE_PATH='+prefix,container,'sh','/task-entrypoint.sh']);
 let ready=false;for(let i=0;i<50;i++){try{const r=await fetch(base+'/healthz');if(r.status===200){ready=true;break;}}catch{}await pause(100);}assert.ok(ready);
 for(const [route,status] of [['/healthz',200],[prefix+'/login',200],[prefix+'/pkg/missing.wasm',404],[prefix+'/missing.js',404],['/_matrix/client/versions',200]]){
  const r=await fetch(base+route);assert.equal(r.status,status,route);assert.equal(r.headers.get('x-frame-options'),'DENY',route);assert.ok(r.headers.get('content-security-policy').includes('wasm-unsafe-eval'));
  const body=await r.text();if(route===prefix+'/login')assert.ok(body.includes('<base href="'+prefix+'/">'));if(route==='/_matrix/client/versions')assert.ok(JSON.parse(body).versions.length);
 }
 if(prefix){const r=await fetch(base+prefix+'?test=1',{redirect:'manual'});assert.equal(r.status,308);assert.equal(r.headers.get('location'),prefix+'/?test=1');}
 const config=await(await fetch(base+prefix+'/runtime-config.js')).text();assert.ok(config.includes("PADMIN_BASE_PATH='"+prefix+"'"));
 results.push((prefix||'root')+': actual nginx nonroot health/redirect/deep-link/runtime-config/strict-static404/security headers/real Matrix proxy');
}
fs.writeFileSync('D:/BuildCache/padmin-glory-branch/nginx-acceptance-results.json',JSON.stringify({results,wasm_sha256:wasm,source_sha256:info.source_sha256,exit_code:0,scope:'Cached nginx executes production entrypoint as nginx user. Source index only, no built WASM copied to C-backed Docker storage. Full app served from D: separately; no image build claim.'},null,2));console.log(results.join('\n'));
})().catch(e=>{console.error(e);process.exitCode=1});
