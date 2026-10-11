# Builds the download: one zip with the installer and everything it installs.
#
#   powershell -ExecutionPolicy Bypass -File scripts\package.ps1
#
# The DAW library is a separate, GPLv3 project. It is included when its
# checkout sits next to this one (..\AxeTwoDriver-DAW); without it the package
# still installs MIDI, the tools and the control panel.

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$daw = Join-Path (Split-Path -Parent $root) 'AxeTwoDriver-DAW'

$version = (Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version = "(.*)"').Matches[0].Groups[1].Value
$name = "AxeTwoDriver-$version-windows-x64"
$out = Join-Path $root "dist\$name"

cargo build --release --locked --manifest-path (Join-Path $root 'Cargo.toml')
if ($LASTEXITCODE -ne 0) { throw 'build failed' }

if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Force $out | Out-Null
foreach ($exe in 'axetwo-setup.exe', 'axetwo.exe', 'axetwo-panel.exe') {
    Copy-Item (Join-Path $root "target\release\$exe") $out
}
Copy-Item (Join-Path $root 'LICENSE') (Join-Path $out 'LICENSE.txt')
Copy-Item (Join-Path $root 'ATTRIBUTION.md') (Join-Path $out 'ATTRIBUTION.txt')
Copy-Item (Join-Path $root 'packaging\README.txt') $out
python (Join-Path $root 'scripts\third-party.py') (Join-Path $out 'THIRD-PARTY-LICENSES.txt')
if ($LASTEXITCODE -ne 0) { throw 'collecting third-party licences failed' }

if (Test-Path $daw) {
    cargo build --release --locked --manifest-path (Join-Path $daw 'Cargo.toml')
    if ($LASTEXITCODE -ne 0) { throw 'DAW library build failed' }
    Copy-Item (Join-Path $daw 'target\release\axetwo_daw.dll') $out
    Copy-Item (Join-Path $daw 'LICENSE') (Join-Path $out 'LICENSE-DAW.txt')
} else {
    Write-Host 'AxeTwoDriver-DAW not found next to this checkout: packaging without the DAW library.'
}

# Which source this download was built from, for both projects.
$source = @("AxeTwoDriver  https://github.com/RamonWide/AxeTwoDriver  commit $(git -C $root rev-parse HEAD)")
if (Test-Path (Join-Path $out 'axetwo_daw.dll')) {
    $source += "AxeTwoDriver-DAW  https://github.com/RamonWide/AxeTwoDriver-DAW  commit $(git -C $daw rev-parse HEAD)"
}
$source | Set-Content -Encoding ascii (Join-Path $out 'SOURCE.txt')

$zip = Join-Path $root "dist\$name.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path $out -DestinationPath $zip
Get-ChildItem $out | Select-Object Name, Length
Write-Host "Wrote $zip"
