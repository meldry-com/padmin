// Historical source assertions only. Keep runtime/cache output on D: and never download implicitly.
const {spawnSync}=require('node:child_process'),fs=require('node:fs');
const cache='D:/BuildCache/padmin-glory-branch';const cli=cache+'/runtime/node_modules/@playwright/test/cli.js';
if(!fs.existsSync(cli))throw Error('Install @playwright/test under D:/BuildCache/padmin-glory/runtime explicitly');
const report=process.argv.includes('show-report');console.log('Historical source baseline. Use npm test / npm run test:real for current Glory acceptance.');
const r=spawnSync(process.execPath,[cli,...(report?process.argv.slice(2):['test',...process.argv.slice(2)])],{stdio:'inherit',windowsHide:true,env:{...process.env,TEMP:cache+'/tmp',TMP:cache+'/tmp',PLAYWRIGHT_BROWSERS_PATH:cache+'/browsers',npm_config_cache:cache+'/npm-cache'}});process.exitCode=r.status??1;
