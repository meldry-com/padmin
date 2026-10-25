# Reuse installed binaries/cached image. No Docker build/pull and no source checkout changes.
param(
    [string]$PasionRoot = 'D:/Works/meldry/pasion',
    [string]$PostgresBin = 'C:/Program Files/PostgreSQL/18/bin',
    [string]$PalpoImage = 'complement-palpo:pr517-ack'
)
$ErrorActionPreference = 'Stop'
$stack = 'D:/BuildCache/meldry-visual-20261025/padmin/real-stack'
$env:TEMP = 'D:/BuildCache/meldry-visual-20261025/padmin/tmp'
$env:TMP = $env:TEMP
New-Item -ItemType Directory -Force $stack,$env:TEMP | Out-Null
$pasion = Join-Path $PasionRoot 'target/debug/pasion.exe'
if (!(Test-Path -LiteralPath $pasion)) { throw "Missing existing Pasion binary: $pasion" }
docker image inspect $PalpoImage *> $null
if ($LASTEXITCODE -ne 0) { throw 'Palpo image must already exist; this script never pulls/builds images.' }
$pgData = Join-Path $stack 'postgres'
if (!(Test-Path -LiteralPath (Join-Path $pgData 'PG_VERSION'))) {
    & "$PostgresBin/initdb.exe" -D $pgData -U padmin_test -A trust --encoding UTF8 --locale C
    if ($LASTEXITCODE -ne 0) { throw 'Postgres initialization failed' }
}
& "$PostgresBin/pg_isready.exe" -h 127.0.0.1 -p 25432 -U padmin_test *> $null
if ($LASTEXITCODE -ne 0) {
    & "$PostgresBin/pg_ctl.exe" -D $pgData -l "$stack/postgres.log" -o '-p 25432 -h 127.0.0.1' start
    if ($LASTEXITCODE -ne 0) { throw 'Postgres start failed' }
}
foreach ($db in @('palpo','pasion')) {
    $exists = & "$PostgresBin/psql.exe" -h 127.0.0.1 -p 25432 -U padmin_test -d postgres -Atc "SELECT 1 FROM pg_database WHERE datname='$db'"
    if ($exists -ne '1') { & "$PostgresBin/createdb.exe" -h 127.0.0.1 -p 25432 -U padmin_test $db }
}
if (!(Test-Path -LiteralPath "$stack/generated.yaml")) {
    & $pasion config generate -o "$stack/generated.yaml"
    if ($LASTEXITCODE -ne 0) { throw 'Pasion config generation failed' }
}
$env:PADMIN_ACCEPTANCE_PASION_ROOT = $PasionRoot
@'
from pathlib import Path
import os,re
p=Path('D:/BuildCache/meldry-visual-20261025/padmin/real-stack')
s=(p/'generated.yaml').read_text()
s=s.replace('uri: postgresql://','uri: postgresql://padmin_test@127.0.0.1:25432/pasion').replace('http://[::]:7080/','http://127.0.0.1:27080/').replace("address: '[::]:7080'","address: '127.0.0.1:27080'")
s=s.replace('    - name: assets','    - name: adminapi\n    - name: assets\n      path: '+os.environ['PADMIN_ACCEPTANCE_PASION_ROOT']+'/dist')
s=s.replace('homeserver: localhost:8008','homeserver: localhost').replace('http://localhost:8008/','http://127.0.0.1:28008/')
s=re.sub(r'  secret: [A-Za-z0-9]+','  secret: acceptance-mas-secret',s)
s+='\ntemplates:\n  path: '+os.environ['PADMIN_ACCEPTANCE_PASION_ROOT']+'/templates\n  translations_path: '+os.environ['PADMIN_ACCEPTANCE_PASION_ROOT']+'/translations\n'
s+='\npolicy:\n  cedar_policy_file: '+os.environ['PADMIN_ACCEPTANCE_PASION_ROOT']+'/policies/cedar/default.cedar\n'
s+='\naccount:\n  password_registration_enabled: true\n'
s+='\nrate_limiting:\n  login:\n    per_ip: {burst: 1000, per_second: 100.0}\n    per_account: {burst: 1000, per_second: 100.0}\n  registration: {burst: 1000, per_second: 100.0}\n  email_authentication:\n    per_ip: {burst: 1000, per_second: 100.0}\n    per_address: {burst: 1000, per_second: 100.0}\n    emails_per_session: {burst: 1000, per_second: 100.0}\n    attempt_per_session: {burst: 1000, per_second: 100.0}\n'
s+='\nclients:\n  - client_id: 01KMQPADM1N000000000000000\n    client_auth_method: none\n    client_name: Glory acceptance\n    redirect_uris:\n      - http://127.0.0.1:28789/oauth/callback\n      - http://127.0.0.1:28790/admin/oauth/callback\n      - http://127.0.0.1:28791/console/manage/oauth/callback\n      - http://127.0.0.1:29980/oauth/callback\n      - http://127.0.0.1:29980/admin/oauth/callback\n      - http://127.0.0.1:29980/console/manage/oauth/callback\n  - client_id: 01KMQE9VZQ33VQT9GP1WA4265X\n    client_auth_method: none\n    client_name: Palpo SSO acceptance\n    redirect_uris:\n      - http://127.0.0.1:28008/_matrix/client/v3/login/sso/callback\n'
(p/'pasion.yaml').write_text(s)
source=Path('examples/palpo.toml').read_text().replace('trusted_servers = ["matrix.org"]','trusted_servers = []').replace('postgres://palpo:changeme@postgres:5432/palpo','postgres://padmin_test@host.docker.internal:25432/palpo').replace('replace-with-a-random-secret','acceptance-mas-secret').replace('http://localhost:7080/','http://127.0.0.1:27080/').replace('http://pasion:7080/oauth2/introspect','http://host.docker.internal:27080/oauth2/introspect')
source+='\n[storage]\nbackend = "fs"\nroot = "/data/media"\n'
source=source.replace('client_id = "01KMQE9VZQ33VQT9GP1WA4265X"','client_id = "01KMQE9VZQ33VQT9GP1WA4265X"\nsso_callback_url = "http://127.0.0.1:28008/_matrix/client/v3/login/sso/callback"\nsso_allowed_redirect_origins = ["http://127.0.0.1:28070", "http://127.0.0.1:28789"]')
(p/'palpo.toml').write_text(source)
'@ | python
if ($LASTEXITCODE -ne 0) { throw 'Config preparation failed' }
& $pasion -c "$stack/pasion.yaml" database migrate
& $pasion -c "$stack/pasion.yaml" config sync
docker container inspect padmin-glory-palpo *> $null
if ($LASTEXITCODE -ne 0) {
    docker create --pull never --name padmin-glory-palpo -p 127.0.0.1:28008:8008 --tmpfs /data --tmpfs /work/space --entrypoint /work/palpo $PalpoImage -c /config.toml
}
docker cp "$stack/palpo.toml" padmin-glory-palpo:/config.toml
docker restart padmin-glory-palpo
# The public issuer remains browser-reachable localhost. Relay that exact origin
# inside this isolated container to the real Windows Pasion server.
docker exec -d padmin-glory-palpo caddy reverse-proxy --from http://127.0.0.1:27080 --to http://host.docker.internal:27080
$listeners = Get-NetTCPConnection -State Listen -LocalPort 27080 -ErrorAction SilentlyContinue
if (!$listeners) {
    $process = Start-Process $pasion -ArgumentList '-c',"$stack/pasion.yaml",'server' -WorkingDirectory $stack -WindowStyle Hidden -RedirectStandardOutput "$stack/pasion.log" -RedirectStandardError "$stack/pasion-error.log" -PassThru
    Set-Content "$stack/pasion.pid" $process.Id
}
$exists = & "$PostgresBin/psql.exe" -h 127.0.0.1 -p 25432 -U padmin_test -d pasion -Atc "SELECT count(*) FROM users WHERE username='fixtureadmin'"
if ($exists -eq '0') {
    & $pasion -c "$stack/pasion.yaml" manage register-user -y --admin --password 'Acceptance-Only-48!6' --email fixture@example.test --display-name 'Glory Acceptance' fixtureadmin
}
Write-Output 'Isolated Palpo:28008, Pasion:27080, PostgreSQL:25432. Run node scripts/real-server.cjs for Padmin:28789.'
