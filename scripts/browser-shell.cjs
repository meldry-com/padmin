const assert=require('node:assert/strict');
const fs=require('node:fs');
const {createHarness,CACHE}=require('./fixture-harness.cjs');
(async()=>{
 const h=await createHarness();const p=h.page;const passed=[];
 const button=name=>p.getByRole('button',{name,exact:true});
 const settle=()=>p.waitForTimeout(120);
 const stats=()=>p.evaluate(async()=>JSON.parse((await import('/pkg/padmin.js')).stats()));
 try{
  await h.login();await p.getByText('Fixture (@fixture:example.test)',{exact:true}).waitFor();await button('Toggle sidebar width').click();await p.locator('.shell.compact').waitFor();await button('Toggle sidebar width').click();await h.navigate('/users/create');
  const editor=p.getByRole('textbox',{name:'Display name',exact:true});
  await editor.fill('🙂 composition test');
  await editor.evaluate(el=>{window.retainedEditor=el;el.focus();el.setSelectionRange(2,5);});
  await button('中文').click();await p.getByRole('heading',{name:'创建用户',exact:true}).waitFor();
  assert.equal(await p.evaluate(()=>window.retainedEditor.isConnected),true);
  assert.equal(await p.evaluate(()=>window.retainedEditor.value),'🙂 composition test');
  assert.deepEqual(await p.evaluate(()=>[window.retainedEditor.selectionStart,window.retainedEditor.selectionEnd]),[2,5]);
  fs.writeFileSync(CACHE+'/create-user-zh.aria.txt',await p.getByRole('main').ariaSnapshot());
  await p.evaluate(()=>{const el=window.retainedEditor;el.dispatchEvent(new CompositionEvent('compositionstart',{bubbles:true,data:''}));el.value='🙂中文输入';el.setSelectionRange(4,4);el.dispatchEvent(new InputEvent('input',{bubbles:true,data:'中文',inputType:'insertCompositionText',isComposing:true}));});
  await settle();assert.equal(await p.evaluate(()=>window.retainedEditor.isConnected&&window.retainedEditor.value==='🙂中文输入'),true);
  await p.evaluate(()=>window.retainedEditor.dispatchEvent(new CompositionEvent('compositionend',{bubbles:true,data:'中文'})));
  await button('English').click();await p.getByRole('heading',{name:'Create user',exact:true}).waitFor();
  passed.push('Language bindings update existing controls, preserving identity, value and UTF-16 selection');
  await button('Dark').click();await p.locator('.padmin.dark').waitFor();
  await button('High contrast').click();await p.locator('.padmin.high-contrast').waitFor();
  await button('High contrast').click();await button('Light').click();await p.locator('.padmin.light').waitFor();
  await editor.focus();await p.keyboard.type('gu');assert.match(p.url(),/users\/create$/);
  await button('Keyboard shortcuts').click();await p.getByRole('dialog',{name:'Keyboard shortcuts',exact:true}).waitFor();
  await p.getByRole('button',{name:'Close',exact:true}).evaluate(el=>{if(document.activeElement!==el)throw Error('dialog focus did not enter');});
  await p.keyboard.press('Tab');assert.equal(await p.getByRole('button',{name:'Close',exact:true}).evaluate(el=>document.activeElement===el),true);
  await p.keyboard.press('Escape');await p.getByRole('dialog').waitFor({state:'detached'});await settle();assert.equal(await button('Keyboard shortcuts').evaluate(el=>document.activeElement===el),true);
  await p.keyboard.type('gu');await p.waitForURL(h.baseURL+'/users');
  passed.push('Theme, contrast, keyboard chords and editing-target exclusion');
  await p.waitForTimeout(5200);assert.match(await p.getByRole('link',{name:'Users',exact:true}).getAttribute('class'),/active/);const baseline=await stats();
  for(let i=0;i<12;i++){await h.navigate('/rooms');await h.navigate('/users');}
  for(let i=0;i<2;i++){for(const path of ['/pasion/accounts','/pasion/upstream-providers','/pasion/personal-sessions','/appservices','/registration-tokens'])await h.navigate(path);await h.navigate('/users');}
  await p.waitForTimeout(5200);const after=await stats();
  for(const key of ['tasks','handlers','queries','owners','nodes'])assert.ok(after[key]<=baseline[key]+3,`${key}: ${baseline[key]} -> ${after[key]}`);
  passed.push('Repeated route disposal restores bounded live task/handler/Owner/node counts');
  h.route('GET','/_palpo/admin/v1/event_reports',()=>({json:{event_reports:[{id:1,received_ts:1,user_id:'@report:example.test',room_id:'!fixture:example.test',event_id:'$fixture',reason:'Fixture review',score:0}],total:1}}));
  await h.navigate('/settings/notifications');
  await p.getByRole('checkbox',{name:'Report filed',exact:true}).uncheck();
  const count=(await stats()).notifications;
  await h.navigate('/reports');await button('Refresh / Retry').click();await settle();
  assert.equal((await stats()).notifications,count);
  await h.navigate('/settings/notifications');await p.getByRole('checkbox',{name:'Report filed',exact:true}).check();
  await h.navigate('/reports');
  for(let i=0;i<55;i++){await button('Refresh / Retry').click();await settle();}
  assert.equal((await stats()).notifications,50);
  passed.push('Notification preferences act immediately; application history stays at 50 entries');
  await p.setViewportSize({width:390,height:844});await settle();
  assert.ok(await p.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+2));
  await button('Toggle navigation').click();await p.locator('.sidebar.open').waitFor();await p.getByRole('link',{name:'Users',exact:true}).click();await p.waitForURL(h.baseURL+'/users');await p.locator('.sidebar.open').waitFor({state:'detached'});
  await button('Toggle navigation').click();await p.getByRole('button',{name:'Close navigation',exact:true}).click({position:{x:385,y:600}});await p.locator('.sidebar.open').waitFor({state:'detached'});
  await p.screenshot({path:CACHE+'/shell-narrow.png',fullPage:true});
  await p.setViewportSize({width:1440,height:1000});
  assert.equal(await p.getByRole('link',{name:'Users',exact:true}).getAttribute('href'),'/users');
  const popup=h.context.waitForEvent('page');await p.getByRole('link',{name:'Rooms',exact:true}).click({modifiers:['Control']});const tab=await popup;await tab.waitForLoadState();assert.match(tab.url(),/rooms$/);await tab.close();assert.match(p.url(),/users$/);
  passed.push('Mobile drawer closes after navigation/backdrop; modified links open a separate tab without changing the current route');
  await button('Keyboard shortcuts').click();await p.getByRole('dialog',{name:'Keyboard shortcuts',exact:true}).waitFor();
  await p.evaluate(async()=>{const app=await import('/pkg/padmin.js');app.unmount();if(app.stats()!=='null')throw Error('not unmounted');app.mount();});
  await p.getByRole('heading',{name:'Sign in required',exact:true}).waitFor();
  await p.goto(h.baseURL+'/unknown-path');await p.getByRole('heading',{name:'Page not found',exact:true}).waitFor();await p.getByText('404',{exact:true}).waitFor();await button('Go to Dashboard').click();await p.waitForURL(h.baseURL+'/');
  passed.push('Narrow layout has no page overflow; unmount removes state and remount requires real authentication');
  h.assertNoErrors();fs.writeFileSync(CACHE+'/browser-shell-results.json',JSON.stringify({passed,baseline,after,errors:h.errors},null,2));console.log(passed.join('\n'));
 }finally{await h.close();}
})().then(()=>process.exit(0)).catch(e=>{console.error(e);process.exit(1);});
