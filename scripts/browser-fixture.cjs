// Run against scripts/glory.py serve. Uses only synthetic OAuth/API fixtures.
const { chromium } = require(process.env.PADMIN_PLAYWRIGHT || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const {closeOwnedBrowser}=require('./fixture-harness.cjs');
(async () => {
 const browserServer = await chromium.launchServer({channel:'msedge',headless:true});
 const browser = await chromium.connect(browserServer.wsEndpoint());
 const results=[]; const prefix=process.env.PADMIN_TEST_PREFIX || ''; const baseURL='http://127.0.0.1:8789'+prefix;
 try {
  const page=await browser.newPage(); const errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  let verdict=200, exchanges=0;
  await page.route('**/runtime-config.js',r=>r.fulfill({contentType:'text/javascript',body:'window.PADMIN_BASE_PATH='+JSON.stringify(prefix)+';window.PADMIN_PASION_BASE_PATH="";'}));
  await page.route('**/config.json',r=>r.fulfill({json:{pasion_public_url:'http://127.0.0.1:8789',oauth_client_id:'fixture-client'}}));
  await page.route('**/_palpo/**',r=>r.fulfill({status:verdict,json:verdict===200?{server_version:'fixture',users:[],rooms:[],event_reports:[],total:0}:{error:'fixture denial'}}));
  await page.route('**/api/admin/v1/**',r=>r.fulfill({status:verdict,json:verdict===200?{data:[],meta:{total:0}}:{errors:[{title:'denied'}]}}));
  await page.route('**/_matrix/client/v3/account/whoami',r=>r.fulfill({json:{user_id:'@fixture:example.test',device_id:'fixture-device'}}));
  await page.route('**/_matrix/client/v3/profile/**',r=>r.fulfill({json:{displayname:'Fixture'}}));
  await page.route('**/oauth2/token',r=>{exchanges++;return r.fulfill({json:{access_token:'synthetic-fixture-token'}});});
  await page.route('**/authorize?**',async r=>{const u=new URL(r.request().url());assert.equal(u.searchParams.get('code_challenge_method'),'S256');assert.ok(u.searchParams.get('code_challenge'));await r.fulfill({status:302,headers:{location:`${u.searchParams.get('redirect_uri')}?code=fixture-code&state=${u.searchParams.get('state')}`}});});
  await page.goto(baseURL+'/users');
  await page.getByText('Sign in required',{exact:true}).waitFor();results.push('Unauthenticated route blocked');
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  await page.getByText('Sign in to Padmin',{exact:true}).waitFor();
  assert.ok(await page.locator('button.sapid-button').count()>0);results.push('Shared application actions render actual Sapid button widgets');
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  await page.getByRole('heading',{name:'Dashboard',exact:true}).waitFor({timeout:20000});
  assert.equal(exchanges,1);results.push('PKCE redirect, one-shot state, token exchange, whoami and admin probes');
  assert.equal(await page.evaluate(()=>localStorage.getItem('access_token')),null);
  assert.equal(await page.evaluate(()=>sessionStorage.getItem('access_token')),null);results.push('Token absent from persistent storage');
  await page.getByRole('link',{name:'Users',exact:true}).click();
  await page.waitForURL('**/users');
  await page.getByRole('button',{name:'Create user',exact:true}).first().waitFor();results.push('Users route mounted');
  await page.goBack();await page.getByRole('heading',{name:'Dashboard',exact:true}).waitFor();results.push('Browser back restores route');
  for(const path of ['/rooms','/media','/reports','/destinations','/registration-tokens','/appservices','/auth-status','/server-status','/server-actions','/server-notices','/server-notifications','/settings/notifications','/pasion/accounts','/pasion/audit-log','/pasion/oauth2-sessions','/pasion/personal-sessions','/pasion/connector-health','/pasion/upstream-providers','/pasion/upstream-links','/pasion/notification-channels','/pasion/notification-templates']) {
   await page.getByRole('link',{name:path.slice(1).split('/').map(s=>s.split('-').map(w=>w[0].toUpperCase()+w.slice(1)).join(' ')).join(' · '),exact:true}).click();
   await page.waitForURL('**'+path);
   await page.waitForTimeout(125);
   assert.equal(await page.locator('.page').count(),1,path);
  }
  results.push('21 additional dedicated route mounts without browser exceptions (responses are fixtures, not backend acceptance)');
  await page.screenshot({path:'D:/BuildCache/meldry-visual-20261025/padmin/browser.png',fullPage:true});
  await page.goto(baseURL+'/oauth/callback?code=replayed&state=bad');
  await page.getByText(/Missing PKCE verifier/).waitFor();assert.equal(exchanges,1);results.push('Callback replay rejected before exchange');
  verdict=403;
  await page.goto(baseURL+'/login');
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  await page.getByText('Not authorized',{exact:true}).waitFor();
  results.push('Server 403 cannot grant administrator access');
  assert.deepEqual(errors,[]);
  fs.writeFileSync('D:/BuildCache/meldry-visual-20261025/padmin/browser-results'+prefix.replaceAll('/','-')+'.json',JSON.stringify({results,errors},null,2));
  console.log(results.join('\n'));
 } finally {await closeOwnedBrowser(browserServer,browser);}
})().then(()=>process.exit(0)).catch(e=>{console.error(e);process.exit(1);});
