$ErrorActionPreference = "Stop"

$Repo = if ($env:CODEX_IMAGE_REPO) { $env:CODEX_IMAGE_REPO } else { "tksuns12/codex-image" }
$InstallDir = if ($env:CODEX_IMAGE_INSTALL_DIR) { $env:CODEX_IMAGE_INSTALL_DIR } else { Join-Path $HOME "bin" }
$ApiUrl = "https://api.github.com/repos/$Repo/releases/latest"

$Release = Invoke-RestMethod $ApiUrl
$Version = $Release.tag_name
if (-not $Version) {
    throw "could not resolve latest codex-image release from $ApiUrl"
}

$Target = "x86_64-pc-windows-msvc"
$Asset = "codex-image-$Version-$Target.zip"
$ArchiveRoot = "codex-image-$Version-$Target"
$TempDir = Join-Path $env:TEMP "codex-image-install-$([System.Guid]::NewGuid().ToString('N'))"
$ZipPath = Join-Path $TempDir $Asset

New-Item -ItemType Directory -Force -Path $TempDir | Out-Null
try {
    Invoke-WebRequest "https://github.com/$Repo/releases/download/$Version/$Asset" -OutFile $ZipPath
    Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    $BinaryPath = Join-Path $InstallDir "codex-image.exe"
    Copy-Item (Join-Path $TempDir "$ArchiveRoot\codex-image.exe") $BinaryPath -Force

    Write-Host "installed codex-image $Version to $BinaryPath"
    Write-Host "make sure $InstallDir is on your PATH"
    & $BinaryPath --help | Out-Null
}
finally {
    if (Test-Path $TempDir) {
        Remove-Item -Recurse -Force $TempDir
    }
}
