# Requires a previously verified D: or remote Docker builder; this script only stages files.
$ErrorActionPreference = 'Stop'
$env:WASM_BINDGEN = 'D:/BuildCache/glory-website/tools/wasm-bindgen-0.2.129-x86_64-pc-windows-msvc/wasm-bindgen.exe'
python "$PSScriptRoot/glory.py" build
if ($LASTEXITCODE -ne 0) { throw 'Build failed' }
$stage = 'D:/BuildCache/meldry-visual-20261025/padmin/container'
New-Item -ItemType Directory -Force -Path $stage | Out-Null
$stagedDist = [IO.Path]::GetFullPath((Join-Path $stage 'dist'))
if ($stagedDist -ne 'D:\BuildCache\meldry-visual-20261025\padmin\container\dist') { throw 'Unexpected staging path' }
if (Test-Path -LiteralPath $stagedDist) { Remove-Item -LiteralPath $stagedDist -Recurse -Force }
Copy-Item -LiteralPath 'D:/BuildCache/meldry-visual-20261025/padmin/dist' -Destination $stagedDist -Recurse -Force
Copy-Item -LiteralPath "$PSScriptRoot/../Dockerfile","$PSScriptRoot/../docker-entrypoint.sh" -Destination $stage -Force
Write-Output "Container context: $stage. Verify builder storage before building."
