// Same-origin host with real backend forwarding; build static output separately.
const http=require('node:http'),fs=require('node:fs'),path=require('node:path');
const root=process.env.PADMIN_DIST||'D:/BuildCache/padmin-glory-branch/dist';
const prefix=(process.env.PADMIN_BASE_PATH||'').replace(/\/$/,'');
const palpo=process.env.PALPO_URL||'http://127.0.0.1:28008';
const pasion=process.env.PASION_URL||'http://127.0.0.1:27080';
const client=process.env.PADMIN_OAUTH_CLIENT_ID||'01KMQPADM1N000000000000000';
const mime={'.js':'text/javascript','.wasm':'application/wasm','.css':'text/css','.json':'application/json','.html':'text/html'};
http.createServer((req,res)=>{
 const url=new URL(req.url,'http://127.0.0.1'),pathname=url.pathname;
 if(pathname==='/healthz'){res.writeHead(200,{'content-type':'text/plain'});return res.end('ok');}
 if(/^\/(_matrix|_palpo|_synapse)\//.test(pathname)||/^\/(api|oauth2|authorize|\.well-known)(\/|$)/.test(pathname)){
  const target=new URL(req.url,/^\/(_matrix|_palpo|_synapse)\//.test(pathname)?palpo:pasion);
  const upstream=http.request(target,{method:req.method,headers:{...req.headers,host:target.host}},r=>{res.writeHead(r.statusCode,r.headers);r.pipe(res);});
  upstream.on('error',()=>{res.writeHead(502);res.end('Backend unavailable');});return req.pipe(upstream);
 }
 if(prefix&&pathname===prefix){res.writeHead(308,{location:prefix+'/'+url.search});return res.end();}
 if(prefix&&!pathname.startsWith(prefix+'/')){res.writeHead(404);return res.end();}
 const relative=prefix?pathname.slice(prefix.length):pathname;
 res.setHeader('cache-control','no-store');
 if(relative==='/config.json'){res.setHeader('content-type','application/json');return res.end(JSON.stringify({pasion_public_url:pasion,oauth_client_id:client}));}
 if(relative==='/runtime-config.js'){res.setHeader('content-type','text/javascript');return res.end('window.PADMIN_BASE_PATH='+JSON.stringify(prefix)+';window.PADMIN_PASION_BASE_PATH="";');}
 let file=path.resolve(root,'.'+relative);
 if(!file.startsWith(path.resolve(root)+path.sep)){res.writeHead(403);return res.end();}
 if(!path.extname(relative))file=path.join(root,'index.html');
 fs.readFile(file,(e,data)=>{if(e){res.writeHead(404);return res.end();}res.setHeader('content-type',mime[path.extname(file)]||'application/octet-stream');if(file.endsWith('index.html'))data=Buffer.from(data.toString().replace(/<base href="[^"]*">/,'<base href="'+prefix+'/">'));res.end(data);});
}).listen(Number(process.env.PORT||28789),'127.0.0.1',()=>console.log('Real backend host ready'));
