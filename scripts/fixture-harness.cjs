// Stateful protocol fixtures. This file never forges an in-memory application session.
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const {spawnSync}=require('node:child_process');
const CACHE='D:/BuildCache/meldry-visual-20261025/padmin';
function playwright(){
  if(process.env.PADMIN_PLAYWRIGHT) return require(process.env.PADMIN_PLAYWRIGHT);
  try { return require('playwright'); } catch {}
  return require('D:/BuildCache/glory-dependencies/browser/node_modules/playwright');
}
async function closeOwnedBrowser(server,browser){
 if(process.platform!=='win32'){await server.kill();await browser.close();return;}
 const pid=server.process().pid;
 const result=spawnSync('taskkill',['/PID',String(pid),'/T','/F'],{windowsHide:true,timeout:15000,encoding:'utf8'});
 let exited=false;
 for(let attempt=0;attempt<100;attempt++){
  try{process.kill(pid,0);}catch(error){if(error.code==='ESRCH'){exited=true;break;}throw error;}
  await new Promise(resolve=>setTimeout(resolve,50));
 }
 assert.ok(exited,'Owned browser process '+pid+' survived native teardown: '+result.stderr);
 console.log('Native browser teardown confirmed PID '+pid+' exited (taskkill '+result.status+')');
 // The native process is positively gone. Do not wait for an SDK cleanup ack
 // that can stall on Windows profile removal; the CLI exits after all assertions.
 void browser.close().catch(()=>{});
}
async function createHarness(options={}) {
 const prefix=options.prefix??process.env.PADMIN_TEST_PREFIX??'';
 const origin=options.origin??'http://127.0.0.1:8789';
 const baseURL=origin+prefix;
 const dir=path.join(CACHE,'browser',crypto.randomUUID()); fs.mkdirSync(dir,{recursive:true});
 process.env.TEMP=path.join(CACHE,'tmp');process.env.TMP=process.env.TEMP;
 const browserServer=await playwright().chromium.launchServer({channel:'msedge',headless:true,downloadsPath:dir});
 const browser=await playwright().chromium.connect(browserServer.wsEndpoint());
 const context=await browser.newContext({acceptDownloads:true,viewport:options.viewport??{width:1440,height:1000}});
 const page=await context.newPage();page.setDefaultTimeout(20000);page.setDefaultNavigationTimeout(20000); const requests=[],errors=[],unhandled=[];
 const state={verdict:200,exchanges:0,authorizations:0,code:null,challenge:null,tokenStatus:200,whoamiStatus:200};
 const handlers=[];
 function route(method,match,handler){handlers.push({method,match,handler});}
 const result={page,context,browser,requests,errors,unhandled,state,baseURL,prefix,route,dir,
  async navigate(p){if(await page.getByRole('dialog').filter({visible:true}).count()){await page.keyboard.press('Escape');}if(p==='/users/create'){await page.getByRole('link',{name:'Users',exact:true}).click();await page.getByRole('button',{name:'Create user',exact:true}).first().click();await page.waitForURL(baseURL+p);return;}const label=p.slice(1).split('/').map(s=>s.split('-').map(w=>w[0]?.toUpperCase()+w.slice(1)).join(' ')).join(' · ');const target=page.getByRole('link',{name:label,exact:true});if(await target.count())await target.click();else await page.goto(baseURL+p);await page.waitForURL(baseURL+p);},
  async login(){await page.goto(baseURL+'/login');await page.getByRole('button',{name:'Sign in',exact:true}).click();await page.getByRole('heading',{name:'Dashboard',exact:true}).waitFor({timeout:20000});assert.equal(state.exchanges,1);},
  async close(){await closeOwnedBrowser(browserServer,browser);},
  assertNoErrors(){assert.deepEqual(errors,[]);},
  writes(){return requests.filter(r=>!['GET','HEAD','OPTIONS'].includes(r.method)&&!r.path.endsWith('/oauth2/token'));},
 };
 page.on('pageerror',e=>errors.push(e.message));
 await context.route('**/*',async r=>{
  const request=r.request(),url=new URL(request.url()),method=request.method();
  let body=request.postData();try{body=JSON.parse(body);}catch{}
  const record={method,path:url.pathname,query:Object.fromEntries(url.searchParams),body,headers:request.headers()};
  if(url.pathname.includes('/_palpo/')||url.pathname.includes('/_matrix/')||url.pathname.includes('/api/')||url.pathname.endsWith('/oauth2/token')) requests.push(record);
  const reply=x=>r.fulfill(x&&('json'in x||'status'in x||'body'in x||'headers'in x)?x:{json:x});
  if(url.pathname.endsWith('/runtime-config.js'))return reply({contentType:'text/javascript',body:'window.PADMIN_BASE_PATH='+JSON.stringify(prefix)+';window.PADMIN_PASION_BASE_PATH="";'});
  if(url.pathname.endsWith('/config.json'))return reply({json:{pasion_public_url:origin,oauth_client_id:'fixture-client'}});
  if(url.pathname.endsWith('/authorize')){
   assert.equal(url.searchParams.get('code_challenge_method'),'S256');assert.equal(url.searchParams.get('client_id'),'fixture-client');
   assert.ok(url.searchParams.get('scope').includes('urn:palpo:admin:*')); assert.ok(url.searchParams.get('scope').includes('urn:pasion:admin'));
   state.authorizations++;state.challenge=url.searchParams.get('code_challenge');state.code='fixture-'+crypto.randomUUID();
   return reply({status:302,headers:{location:`${url.searchParams.get('redirect_uri')}?code=${state.code}&state=${url.searchParams.get('state')}`}});
  }
  if(url.pathname.endsWith('/oauth2/token')){
   const form=new URLSearchParams(request.postData());
   if(form.get('grant_type')==='authorization_code'){
    assert.equal(form.get('code'),state.code);assert.equal(form.get('client_id'),'fixture-client');assert.equal(crypto.createHash('sha256').update(form.get('code_verifier')??'').digest('base64url'),state.challenge);state.code=null;
   }
   state.exchanges++;return reply({status:state.tokenStatus,json:state.tokenStatus===200?{access_token:'synthetic-fixture-token',refresh_token:'synthetic-fixture-refresh',expires_in:3600}:{error:'invalid_grant'}});
  }
  for(const h of [...handlers].reverse())if((h.method==='*'||h.method===method)&&(typeof h.match==='string'?h.match===url.pathname:h.match.test(url.pathname)))return reply(await h.handler(record,result));
  if(url.pathname.endsWith('/_matrix/client/v3/account/whoami'))return reply({status:state.whoamiStatus,json:state.whoamiStatus===200?{user_id:'@fixture:example.test',device_id:'fixture-device'}:{errcode:'M_UNKNOWN_TOKEN',error:'fixture denied'}});
  if(url.pathname.includes('/_matrix/client/v3/profile/'))return reply({json:{displayname:'Fixture'}});
  if(url.pathname.endsWith('/server_version')||url.pathname.endsWith('/api/admin/v1/version'))return reply({status:state.verdict,json:state.verdict===200?{server_version:'fixture',version:'fixture'}:{error:'fixture denial'}});
  if(url.pathname.includes('/_palpo/')||url.pathname.includes('/api/admin/')){
   if(method!=='GET'){unhandled.push(record);return reply({status:501,json:{errcode:'M_UNRECOGNIZED',error:'Unregistered fixture mutation'}});}
   return reply({json:{data:[],meta:{count:0,total:0},users:[],rooms:[],event_reports:[],destinations:[],registration_tokens:[],total:0}});
  }
  return r.continue();
 });
 for(const h of options.routes??[])route(h.method,h.path,h.handler);
 return result;
}
module.exports={createHarness,closeOwnedBrowser,CACHE};
