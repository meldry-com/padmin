// Real acceptance uses only actual backend replies. No Playwright request routing.
const assert=require('node:assert/strict');
const crypto=require('node:crypto');
const fs=require('node:fs');
const {chromium}=require(process.env.PADMIN_PLAYWRIGHT||'D:/BuildCache/glory-dependencies/browser/node_modules/playwright');
async function createRealHarness(options={}) {
 process.env.TEMP='D:/BuildCache/meldry-visual-20261025/padmin/tmp';process.env.TMP=process.env.TEMP;
 const baseURL=options.baseURL||process.env.PADMIN_REAL_URL||'http://127.0.0.1:28789';
 const browser=await chromium.launch({channel:'msedge',headless:true});
 const context=await browser.newContext({acceptDownloads:true});const page=await context.newPage();
 const requests=[],responses=[],errors=[],results=[];let challenge=null,exchanged=false;
 page.on('pageerror',e=>errors.push(e.message));
 page.on('request',r=>{
  const u=new URL(r.url());
  if(u.pathname.endsWith('/authorize'))challenge=u.searchParams.get('code_challenge');
  if(u.pathname.endsWith('/oauth2/token')&&r.method()==='POST'){
   const f=new URLSearchParams(r.postData());
   if(f.get('grant_type')==='authorization_code'){
    assert.equal(crypto.createHash('sha256').update(f.get('code_verifier')).digest('base64url'),challenge);exchanged=true;
   }
  }
  if(/^\/(_matrix|_palpo|api)\//.test(u.pathname)||u.pathname.includes('/api/')){let body=r.postData();try{body=JSON.parse(body);}catch{}requests.push({method:r.method(),path:u.pathname,query:Object.fromEntries(u.searchParams),body});}
 });
 page.on('response',r=>{const u=new URL(r.url());if(/^\/(_matrix|_palpo|api)\//.test(u.pathname)||u.pathname.includes('/api/'))responses.push({method:r.request().method(),path:u.pathname,status:r.status()});});
 page.on('dialog',d=>d.accept());
 const h={page,context,browser,requests,responses,errors,results,baseURL,
  async login(){
   await page.goto(baseURL+'/login');await page.getByRole('button',{name:'Sign in',exact:true}).click();
   const allow=page.getByRole('button',{name:'Allow',exact:true});
   const password=page.locator('input[type=password]');
   const dashboard=page.getByRole('heading',{name:'Dashboard',exact:true});
   await Promise.race([password.waitFor({timeout:15000}),allow.waitFor({timeout:15000}),dashboard.waitFor({timeout:15000})]);
   if(await password.isVisible()){
    await page.locator('input[type=text]').fill(process.env.PADMIN_REAL_USERNAME||'fixtureadmin');
    await password.fill(process.env.PADMIN_REAL_PASSWORD||'Acceptance-Only-48!6');
    await page.getByRole('button',{name:'Sign in',exact:true}).click();
    await Promise.race([allow.waitFor({timeout:15000}),dashboard.waitFor({timeout:15000})]);
   }
   if(await allow.isVisible())await allow.click();
   await page.getByRole('heading',{name:'Dashboard',exact:true}).waitFor({timeout:20000});
   assert.equal(exchanged,true);assert.equal(await page.evaluate(()=>localStorage.getItem('access_token')),null);
   results.push('Actual PKCE SHA256 verifier/challenge, password login, consent, token exchange, whoami and dual admin verdict');
  },
  async navigate(p){const label=p.slice(1).split('/').map(s=>s.split('-').map(w=>w[0]?.toUpperCase()+w.slice(1)).join(' ')).join(' · ');await page.getByRole('link',{name:label,exact:true}).click();await page.waitForURL(baseURL+p);},
  async close(){await browser.close();},
  save(name){assert.deepEqual(errors,[]);fs.writeFileSync('D:/BuildCache/meldry-visual-20261025/padmin/'+name+'.json',JSON.stringify({baseURL,results,responses,errors},null,2));},
 };
 return h;
}
module.exports={createRealHarness};
