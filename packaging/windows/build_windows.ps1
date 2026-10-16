# Builds the Windows downloads (run on Windows from anywhere):
#   dist\bounce_tales-<version>-windows-x64.zip        portable: the game, README, licence
#   dist\bounce_tales-<version>-windows-x64-setup.exe  installer (Inno Setup 6)
# Inno Setup is installed with Chocolatey if it is missing.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..\..')

$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"' |
    Select-Object -First 1).Matches[0].Groups[1].Value

cargo build --release
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
New-Item -ItemType Directory -Force dist | Out-Null

# Portable zip: one folder with the game in it.
$stage = 'target\portable\Bounce Tales'
Remove-Item -Recurse -Force target\portable -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item target\release\bounce_tales.exe $stage
Copy-Item README.md $stage
Copy-Item LICENSE (Join-Path $stage 'LICENSE.txt')
$zip = "dist\bounce_tales-$version-windows-x64.zip"
Remove-Item $zip -ErrorAction SilentlyContinue
Compress-Archive -Path $stage -DestinationPath $zip

# Installer.
$iscc = (Get-Command iscc -ErrorAction SilentlyContinue).Source
if (-not $iscc) { $iscc = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe" }
if (-not (Test-Path $iscc)) {
    choco install innosetup -y --no-progress
    if ($LASTEXITCODE -ne 0) { throw 'could not install Inno Setup' }
}
& $iscc "/DAppVersion=$version" packaging\windows\installer.iss
if ($LASTEXITCODE -ne 0) { throw 'iscc failed' }

Write-Output $zip "dist\bounce_tales-$version-windows-x64-setup.exe"
