const {createRealHarness}=require('./real-browser-harness.cjs');
const assert=require('node:assert/strict');
(async()=>{const h=await createRealHarness();const p=h.page;const btn=n=>p.getByRole('button',{name:n,exact:true}),field=n=>p.getByRole('textbox',{name:n,exact:true});
const suffix=Date.now().toString(36),username='acceptance'+suffix;
async function status(s){await p.locator('.page .status').filter({hasText:s}).first().waitFor({timeout:15000});}
try{
await h.login();await h.navigate('/users');await field('Search users').fill('fixtureadmin');await btn('Search / refresh').click();await p.locator('.record').filter({hasText:'@fixtureadmin:localhost'}).waitFor();await p.locator('.record').filter({hasText:'@fixtureadmin:localhost'}).getByRole('button',{name:'User details',exact:true}).click();await btn('Save profile').waitFor();await status('Profile loaded');

await field('Display name').fill('Glory Acceptance Matrix');await btn('Save profile').click();await status('Profile saved');
for(const [button,text] of [['Load account data','Account data loaded'],['Load rate limits','Rate limits loaded'],['Load / refresh joined rooms','joined rooms'],['Load / refresh devices','devices'],['Load / refresh sessions','Sessions loaded']]){
 if(await btn(button).count()){await btn(button).click();await status(text);h.results.push('Real user panel: '+button);}
}
await btn('Load / refresh features').click();await status('404');h.results.push('Unsupported experimental features returns actual typed404 rather than success');h.results.push('Actual user search, encoded identity detail and profile update');
await h.navigate('/users/create');await btn('Create user').click();await status('Username is required');await field('Username').fill(username);await field('Display name').fill('Acceptance disposable');await p.getByLabel('Password',{exact:true}).fill('Acceptance-User-95!3');await field('Email').fill(username+'@example.test');await btn('Create user').click();await p.waitForURL(/\/users\//,{timeout:15000});await status('Profile loaded');
await field('Messages per second (blank uses default)').fill('3');await field('Burst count (blank uses default)').fill('6');await btn('Save rate limits').click();await status('Rate limits saved');await btn('Load rate limits').click();await status('Rate limits loaded');
await field('Messages per second (blank uses default)').fill('');await field('Burst count (blank uses default)').fill('');await btn('Save rate limits').click();await status('Rate limits saved');
await field('New email or phone address').fill(username+'second@example.test');await btn('Add identifier').click();await status('Identifier added');await p.locator('.record').filter({hasText:username+'second@example.test'}).getByRole('button',{name:'Remove identifier',exact:true}).click();await status('Identifier removed');
await btn('Deactivate account').click();await status('Account change accepted');await p.getByText(/Deactivated: true/).first().waitFor();await btn('Reactivate account').click();await status('Account change accepted');await p.getByText(/Deactivated: false/).first().waitFor();
h.results.push('Actual user creation, rate-limit set/reset, identifier add/remove, Pasion-owned deactivate/reactivate and fresh state');
const names=[username,username+'b'];
await h.navigate('/users/create');await field('Username').fill(names[1]);await field('Display name').fill('Acceptance second');await p.getByLabel('Password',{exact:true}).fill('Acceptance-User-95!3');await btn('Create user').click();await p.waitForURL(/\/users\//);await status('Profile loaded');
for(const name of names){await h.navigate('/users');await field('Search users').fill(name);await btn('Search / refresh').click();await p.locator('.record').filter({hasText:'@'+name+':localhost'}).waitFor();}h.results.push('Two independently created users are each found through actual server-backed list searches');

await h.navigate('/rooms');await field('Room name').fill('Acceptance room '+suffix);await field('Room topic').fill('Acceptance topic');await btn('Create room').click();await p.waitForURL(/\/rooms\//,{timeout:15000});await btn('Save room name').waitFor();await status('Room loaded');
await field('Room name').fill('Renamed '+suffix);await btn('Save room name').click();await status('Room name saved');await field('Room topic').fill('Edited topic 测试');await btn('Save room topic').click();await status('Room topic saved');
h.results.push('Actual create-room and name/topic state writes followed by fresh detail');
await btn('Load directory visibility').click();await status('Directory visibility loaded');await p.getByRole('checkbox',{name:'Published in room directory'}).check();await btn('Save directory visibility').click();await status('Directory visibility saved');
for(const [name,value,button,msg] of [['Join rule: public, invite, knock, restricted, knock_restricted','invite','Save join rule','Join rule saved'],['History visibility: world_readable, shared, invited, joined','shared','Save history visibility','History visibility saved']]){await field(name).fill(value);await btn(button).click();await status(msg);}
h.results.push('Actual room directory/join-rule/history visibility writes');
await field('Room alias (#name:server)').fill('#'+username+':localhost');await btn('Add alias').click();await status('Alias added');await btn('Remove alias').click();await status('Alias removed');h.results.push('Actual alias add/remove');
await field('Member user ID (@user:server)').fill('@'+username+':localhost');
for(const title of ['Invite member','Kick member','Invite member','Ban member','Unban member']){await btn(title).click();await status('Member operation completed');}
await btn('Block room').click();await status('Room blocked');await btn('Unblock room').click();await status('Room unblocked');h.results.push('Actual invite/kick/ban/unban and block/unblock changes');
await btn('Load / refresh members').click();await status('members');await btn('Load / refresh state').click();await status('state events');await btn('Load / refresh messages').click();await status('messages');
await h.navigate('/users/known');await field('Search known users').fill('fixtureadmin');await btn('Search / refresh').click();await btn('Known user details').first().click();await p.getByRole('heading',{name:/Known user/}).waitFor();h.results.push('Actual known-user membership detail');
await btn('Sign out').click();await p.getByRole('heading',{name:'Sign in to Padmin'}).waitFor();h.results.push('Actual logout clears private application session');h.save('browser-real-matrix-results');console.log(h.results.join('\n'));
}catch(e){console.log('PAGE',await p.locator('.page').innerText().catch(()=>''));console.log('RESPONSES',JSON.stringify(h.responses.slice(-8)));throw e;}finally{await h.close();}
})().catch(e=>{console.error(e);process.exitCode=1});
