const assert=require('node:assert/strict');const fs=require('node:fs');
const {createHarness,CACHE}=require('./fixture-harness.cjs');
(async()=>{const passed=[];
 let caseNumber=0;
 async function run(test){const id=++caseNumber;console.log('Auth case '+id+' starting');const h=await createHarness();try{await test(h);h.assertNoErrors();console.log('Auth case '+id+' assertions complete');}finally{console.log('Auth case '+id+' closing');await h.close();console.log('Auth case '+id+' closed');}}
 await run(async h=>{
  const p=h.page;let fails=true;
  await p.route('**/config.json',r=>r.fulfill(fails?{status:500,json:{error:'configuration unavailable'}}:{json:{pasion_public_url:'http://127.0.0.1:8789',oauth_client_id:'fixture-client'}}));
  await p.goto(h.baseURL+'/login');await p.getByText('Unable to load config.json',{exact:true}).waitFor();
  assert.equal(await p.getByRole('button',{name:'Sign in',exact:true}).count(),0);
  fails=false;await p.getByRole('button',{name:'Retry configuration',exact:true}).click();
  await p.getByRole('heading',{name:'Sign in to Padmin',exact:true}).waitFor();passed.push('Configuration failure blocks login and supports explicit successful retry');
 });
 await run(async h=>{
  const p=h.page;h.state.whoamiStatus=401;
  await p.goto(h.baseURL+'/login');await p.getByRole('button',{name:'Sign in',exact:true}).click();
  await p.getByText(/Matrix whoami failed/).waitFor();
  assert.equal(h.requests.filter(r=>r.path.endsWith('/server_version')).length,0);
  assert.equal(await p.evaluate(()=>localStorage.getItem('access_token')),null);
  passed.push('Failed whoami never publishes an administrator session');
 });
 await run(async h=>{
  const p=h.page;
  await p.route('**/authorize?**',r=>{const u=new URL(r.request().url());return r.fulfill({status:302,headers:{location:u.searchParams.get('redirect_uri')+'?code=untrusted&state=mismatch'}});});
  await p.goto(h.baseURL+'/login');await p.getByRole('button',{name:'Sign in',exact:true}).click();
  await p.getByText(/OAuth state mismatch/).waitFor();assert.equal(h.state.exchanges,0);
  await p.goto(h.baseURL+'/oauth/callback?code=replay&state=mismatch');await p.getByText(/Missing PKCE verifier/).waitFor();assert.equal(h.state.exchanges,0);
  passed.push('Mismatched state consumes one-shot verifier and rejects token exchange/replay');
 });
 await run(async h=>{
  const p=h.page;let delayed=false;
  await p.route('**/_matrix/client/v3/profile/**',async r=>{delayed=true;await new Promise(resolve=>setTimeout(resolve,600));await r.fulfill({json:{displayname:'Delayed profile'}});});
  await h.login();assert.equal(delayed,true);assert.equal(h.state.exchanges,1);passed.push('Slow profile and concurrent administrator probes cannot remount/replay OAuth callback');
 });
 await run(async h=>{
  const p=h.page;await h.login();let expired=true,refreshes=0;
  await p.route('**/oauth2/token',async r=>{
   const form=new URLSearchParams(r.request().postData());if(form.get('grant_type')!=='refresh_token')return r.fallback();
   refreshes++;await new Promise(resolve=>setTimeout(resolve,200));expired=false;await r.fulfill({json:{access_token:'refreshed-synthetic',refresh_token:'rotated-synthetic'}});
  });
  h.route('GET',/\/_palpo\//,r=>expired?{status:401,json:{errcode:'M_UNKNOWN_TOKEN',error:'expired fixture'}}:{json:{server_version:'fixture',users:[],rooms:[],event_reports:[],total:0,total_rooms:0}});
  const refreshed=p.waitForResponse(r=>new URL(r.url()).pathname==='/oauth2/token'&&new URLSearchParams(r.request().postData()).get('grant_type')==='refresh_token');
  await p.getByRole('button',{name:'Refresh statistics',exact:true}).click();
  await refreshed;
  await p.locator('.page .status').filter({hasText:'Statistics refreshed'}).waitFor();
  assert.equal(refreshes,1);assert.equal(h.requests.filter(r=>r.method==='GET'&&r.headers.authorization==='Bearer refreshed-synthetic').length>=2,true);
  passed.push('Concurrent 401s coalesce into one refresh and retry using the rotated access token');
  await h.navigate('/users');await p.getByRole('main').getByText('No users found',{exact:true}).first().waitFor();await p.waitForTimeout(250);h.state.verdict=403;
  h.route('GET','/_palpo/admin/v2/users',()=>({status:403,json:{errcode:'M_FORBIDDEN',error:'revoked fixture'}}));
  h.route('GET','/_palpo/admin/v1/server_version',()=>({status:403,json:{errcode:'M_FORBIDDEN',error:'revoked fixture'}}));
  await p.getByRole('button',{name:'Search / refresh',exact:true}).click();await p.getByRole('heading',{name:'Not authorized',exact:true}).waitFor();
  passed.push('A later forbidden response rechecks server permission and removes protected UI');
 });
 await run(async h=>{
  const p=h.page;await h.login();h.state.tokenStatus=401;
  h.route('GET','/_palpo/admin/v2/users',()=>({status:401,json:{errcode:'M_UNKNOWN_TOKEN',error:'expired'}}));
  await h.navigate('/users');await p.getByRole('heading',{name:'Sign in required',exact:true}).waitFor();
  assert.equal(h.state.exchanges,2);passed.push('Rejected refresh retires the session and removes the management page');
 });
 await run(async h=>{
  const p=h.page;await h.login();let started;const refreshing=new Promise(resolve=>started=resolve);
  h.route('POST','/oauth2/revoke',()=>({json:{}}));h.route('POST','/api/v1/auth/logout',()=>({json:{}}));
  h.route('GET','/_palpo/admin/v2/users',()=>({status:401,json:{errcode:'M_UNKNOWN_TOKEN',error:'expired'}}));
  await p.route('**/oauth2/token',async r=>{if(new URLSearchParams(r.request().postData()).get('grant_type')!=='refresh_token')return r.fallback();started();await new Promise(resolve=>setTimeout(resolve,600));await r.fulfill({json:{access_token:'obsolete-refresh',refresh_token:'obsolete-refresh-token'}}).catch(()=>{});});
  await h.navigate('/users');await refreshing;await p.getByRole('button',{name:'Sign out',exact:true}).click();
  await p.getByRole('heading',{name:'Sign in to Padmin',exact:true}).waitFor();await p.waitForTimeout(800);
  await h.navigate('/users');await p.getByRole('heading',{name:'Sign in required',exact:true}).waitFor();passed.push('Logout revokes publication immediately; a late refresh cannot restore authentication');
 });
 await run(async h=>{
  const p=h.page;h.state.verdict=500;
  await p.goto(h.baseURL+'/login');await p.getByRole('button',{name:'Sign in',exact:true}).click();
  await p.getByText(/admin probe HTTP 500/).waitFor();assert.equal(await p.getByRole('heading',{name:'Dashboard',exact:true}).count(),0);
  h.state.verdict=200;await p.getByRole('button',{name:'Retry',exact:true}).click();await p.getByRole('heading',{name:'Dashboard',exact:true}).waitFor();assert.equal(h.state.exchanges,1);passed.push('A failed administrator probe grants nothing; explicit retry reuses the live session without replaying OAuth');
 });
 await run(async h=>{
  const p=h.page;await h.login();h.state.verdict=403;
  await p.getByRole('heading',{name:'Not authorized',exact:true}).waitFor({timeout:36000});
  passed.push('The actual periodic heartbeat detects server-side revocation without a foreground request');
 });
 await run(async h=>{
  const p=h.page;await p.addInitScript(()=>Object.defineProperty(window,'localStorage',{get(){throw new DOMException('Disabled storage','SecurityError');}}));
  await h.login();await p.getByRole('heading',{name:'Dashboard',exact:true}).waitFor();passed.push('Unavailable persistent storage does not crash the app or prevent runtime-config OAuth');
 });
 await run(async h=>{
  const p=h.page;await h.login();h.route('POST','/api/v1/auth/logout',()=>({json:{}}));
  const other=await h.context.newPage();await other.goto(h.baseURL+'/login');await other.getByRole('button',{name:'Sign in',exact:true}).click();await other.getByRole('heading',{name:'Dashboard',exact:true}).waitFor();await other.getByRole('button',{name:'Sign out',exact:true}).click();
  await p.getByRole('heading',{name:'Sign in to Padmin',exact:true}).waitFor();
  await h.navigate('/users');await p.getByRole('heading',{name:'Sign in required',exact:true}).waitFor();await other.close();passed.push('An actual storage event from another tab retires this app session without sharing tokens');
 });
 fs.writeFileSync(CACHE+'/browser-auth-results.json',JSON.stringify({passed},null,2));console.log(passed.join('\n'));
})().then(()=>process.exit(0)).catch(e=>{console.error(e);process.exit(1);});
