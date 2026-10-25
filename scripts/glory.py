"""Glory build entry: all compiler/download/temp output is explicitly on D:."""
import argparse
import json
import hashlib
import os
import platform
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
WSL = os.name != 'nt' and 'microsoft' in platform.release().lower()
REMOTE = os.environ.get('PADMIN_REMOTE_BUILD') == '1' and os.name != 'nt' and not WSL
requested_cache = Path(os.environ.get('PADMIN_BUILD_ROOT', '/mnt/d/BuildCache/padmin-glory-branch' if WSL else 'D:/BuildCache/padmin-glory-branch'))
if REMOTE and not requested_cache.is_absolute():
    raise RuntimeError('Remote CI requires an absolute PADMIN_BUILD_ROOT')
CACHE = requested_cache.resolve()
if WSL:
    if not CACHE.is_relative_to(Path('/mnt/d/BuildCache')):
        raise RuntimeError('WSL builds must use /mnt/d/BuildCache')
    storage = subprocess.check_output(['findmnt', '--noheadings', '--output', 'SOURCE', '--target', '/mnt/d'], text=True).strip()
    if not storage.lower().startswith('d:'):
        raise RuntimeError(f'WSL /mnt/d is not verified as the D: drive: {storage}')
elif not REMOTE and CACHE.drive.lower() != 'd:':
    raise RuntimeError('Local builds must use a D: build root; remote CI must explicitly set PADMIN_REMOTE_BUILD=1')
os.environ.update(CARGO_HOME=str(CACHE / 'cargo') if REMOTE else ('/mnt/d/BuildCache/cargo' if WSL else 'D:/BuildCache/cargo'), CARGO_TARGET_DIR=str(CACHE / 'target'), TEMP=str(CACHE / 'tmp'), TMP=str(CACHE / 'tmp'), TMPDIR=str(CACHE / 'tmp'))
for path in [CACHE / 'tmp', CACHE / 'target', Path(os.environ['CARGO_HOME'])]:
    path.mkdir(parents=True, exist_ok=True)

def run(*args, capture=False):
    return subprocess.run(args, cwd=ROOT, check=True, text=True, capture_output=capture)

def source_digest():
    files = [p for directory in ['src', 'locales'] for p in (ROOT / directory).rglob('*') if p.is_file()]
    files += [ROOT / name for name in ['Cargo.toml', 'Cargo.lock', 'bootstrap.js', 'pasion-config.js', 'index.html']]
    # Capture local framework/widget changes as build inputs as well as app files.
    for library in ['glory', 'sapid']:
        base = (ROOT / '../../glory-rs' / library).resolve()
        files.append(base / 'Cargo.toml')
        if (base / 'Cargo.lock').is_file(): files.append(base / 'Cargo.lock')
        for directory in (['crates'] if library == 'glory' else ['src']):
            files += [p for p in (base / directory).rglob('*') if p.is_file() and (p.suffix == '.rs' or p.name == 'Cargo.toml')]
    files = sorted(set(files), key=lambda p: os.path.relpath(p, ROOT).replace('\\', '/'))
    digest = hashlib.sha256()
    for path in files:
        digest.update(os.path.relpath(path, ROOT).replace('\\', '/').encode())
        digest.update(b'\0')
        digest.update(path.read_bytes())
        digest.update(b'\0')
    return digest.hexdigest(), [os.path.relpath(p, ROOT).replace('\\', '/') for p in files]

def write_public_config(output, pasion_public_url='', oauth_client_id='01KMQPADM1N000000000000000', palpo_admin_url=''):
    output.mkdir(parents=True, exist_ok=True)
    (output/'config.json').write_text(json.dumps({
        'pasion_public_url': pasion_public_url,
        'oauth_client_id': oauth_client_id,
        'palpo_admin_url': palpo_admin_url,
    }, ensure_ascii=False), encoding='utf-8')

def serve(output, base_path, port):
    import http.server
    class Handler(http.server.SimpleHTTPRequestHandler):
        def __init__(self,*a,**kw): super().__init__(*a,directory=str(output),**kw)
        def do_GET(self):
            prefix=base_path.rstrip('/')
            if prefix and (self.path == prefix or self.path.startswith(prefix+'/')):
                self.path=self.path[len(prefix):] or '/'
            if not any(self.path.split('?')[0].endswith(ext) for ext in ['.css','.js','.wasm','.json']):
                self.path='/index.html'
            return super().do_GET()
    http.server.ThreadingHTTPServer(('127.0.0.1',port),Handler).serve_forever()

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('command', choices=['metadata','check','test','build','fmt','serve','fingerprint'])
    parser.add_argument('--port',type=int,default=8080)
    parser.add_argument('--dev',action='store_true',help='Build debug WASM for faster local iteration')
    parser.add_argument('--base-path',default='')
    parser.add_argument('--pasion-public-url', default=os.environ.get('PASION_PUBLIC_URL',''))
    parser.add_argument('--oauth-client-id', default=os.environ.get('PADMIN_OAUTH_CLIENT_ID','01KMQPADM1N000000000000000'))
    parser.add_argument('--palpo-admin-url', default=os.environ.get('PALPO_ADMIN_URL',''))
    args=parser.parse_args()
    metadata=json.loads(run('cargo','metadata','--no-deps','--format-version','1',capture=True).stdout)
    actual=Path(metadata['target_directory']).resolve()
    expected=(CACHE/'target').resolve()
    if actual != expected or (not REMOTE and not WSL and actual.drive.lower() != 'd:'):
        raise RuntimeError(f'Unexpected Cargo target directory: {actual}')
    print(f'Verified target_directory={actual}',flush=True)
    if args.command=='metadata': return
    if args.command=='fingerprint':
        digest, files = source_digest()
        print(json.dumps({'source_sha256': digest, 'source_files': files}))
        return
    if args.command=='check': run('cargo','check','--all-targets'); run('cargo','check','--target','wasm32-unknown-unknown')
    elif args.command=='test': run('cargo','test','--lib')
    elif args.command=='fmt': run('cargo','fmt','--package','padmin','--','--check')
    else:
        source_before, files = source_digest()
        build_args=['cargo','build','--target','wasm32-unknown-unknown']
        if not args.dev: build_args.append('--release')
        run(*build_args)
        output=CACHE/'dist'; output.mkdir(exist_ok=True)
        run(os.environ.get('WASM_BINDGEN', 'wasm-bindgen'),str(expected/'wasm32-unknown-unknown'/('debug' if args.dev else 'release')/'padmin.wasm'),'--target','web','--out-dir',str(output/'pkg'))
        (output/'index.html').write_text((ROOT/'index.html').read_text().replace('<base href="/">', '<base href="'+args.base_path.rstrip('/')+'/">'))
        shutil.copy(ROOT/'src/style.css',output/'style.css')
        shutil.copy(ROOT/'bootstrap.js',output/'bootstrap.js')
        shutil.copy(ROOT/'pasion-config.js',output/'pasion-config.js')
        (output/'runtime-config.js').write_text('window.PADMIN_BASE_PATH='+json.dumps(args.base_path.rstrip('/'))+';window.PADMIN_PASION_BASE_PATH="";')
        write_public_config(output,args.pasion_public_url,args.oauth_client_id,args.palpo_admin_url)
        source_after, _ = source_digest()
        if source_after != source_before:
            raise RuntimeError('Application sources changed during the build; rebuild before acceptance')
        (output/'build-info.json').write_text(json.dumps({
            'source_sha256': source_after,
            'source_files': files,
            'wasm_sha256': hashlib.sha256((output/'pkg/padmin_bg.wasm').read_bytes()).hexdigest(),
            'profile': 'debug' if args.dev else 'release',
        }, indent=2))
        if args.command=='serve':
            serve(output,args.base_path,args.port)
        print(f'Artifacts: {output}')
if __name__=='__main__': main()
