param(
    [string]$Target = "",
    [string]$Profile = "release",
    [string]$OutputDir = "dist"
)

$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
Set-Location $RootDir

function Get-WorkspaceVersion {
    $inSection = $false
    foreach ($line in Get-Content (Join-Path $RootDir "Cargo.toml")) {
        if ($line -match '^\[workspace\.package\]$') {
            $inSection = $true
            continue
        }

        if ($inSection -and $line -match '^\[') {
            break
        }

        if ($inSection -and $line -match '^version = "(.+)"$') {
            return $Matches[1]
        }
    }

    throw "Failed to read workspace version from Cargo.toml."
}

function Get-HostTarget {
    foreach ($line in rustc -vV) {
        if ($line -match '^host:\s+(.+)$') {
            return $Matches[1]
        }
    }

    throw "Failed to determine Rust host target."
}

if (-not $Target) {
    $Target = Get-HostTarget
}

if (-not $Target) {
    throw "Target triple must not be empty."
}

$Version = Get-WorkspaceVersion
$BinNames = @("skillctrl", "skillctrl-desktop")
$BinExt = if ($Target -like "*windows*") { ".exe" } else { "" }
$DesktopDir = Join-Path $RootDir "skillctrl-desktop"

if (-not (Test-Path $DesktopDir)) {
    throw "Desktop app directory not found: $DesktopDir"
}

Write-Host "Preparing skillctrl-desktop frontend..."
Push-Location $DesktopDir
try {
    if (-not (Test-Path (Join-Path $DesktopDir "node_modules"))) {
        & npm ci --no-fund --no-audit
    }
    & npm run build
}
finally {
    Pop-Location
}

$BuildArgs = @("build", "--locked", "--target", $Target)
foreach ($BinName in $BinNames) {
    $BuildArgs += @("--package", $BinName)
}
if ($Profile -eq "release") {
    $BuildArgs += "--release"
} else {
    $BuildArgs += @("--profile", $Profile)
}

Write-Host "Building $($BinNames -join ', ') $Version for $Target ($Profile)..."
& cargo @BuildArgs

$BinaryPaths = @()
foreach ($BinName in $BinNames) {
    $BinaryPath = Join-Path $RootDir "target/$Target/$Profile/$BinName$BinExt"
    if (-not (Test-Path $BinaryPath)) {
        throw "Expected binary not found: $BinaryPath"
    }
    $BinaryPaths += $BinaryPath
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$ResolvedOutputDir = (Resolve-Path $OutputDir).Path

$PackageBase = "skillctrl-v$Version-$Target"
$ArchivePath = Join-Path $ResolvedOutputDir "$PackageBase.zip"
$ChecksumPath = "$ArchivePath.sha256"

$WorkDir = Join-Path ([System.IO.Path]::GetTempPath()) ("skillctrl-package-" + [System.Guid]::NewGuid().ToString("N"))
$PackageDir = Join-Path $WorkDir $PackageBase
New-Item -ItemType Directory -Force -Path $PackageDir | Out-Null

try {
    foreach ($BinName in $BinNames) {
        $BinaryPath = Join-Path $RootDir "target/$Target/$Profile/$BinName$BinExt"
        Copy-Item $BinaryPath (Join-Path $PackageDir "$BinName$BinExt")
    }

    if (Test-Path (Join-Path $RootDir "README.md")) {
        Copy-Item (Join-Path $RootDir "README.md") $PackageDir
    }

    if (Test-Path (Join-Path $RootDir "USER_GUIDE.md")) {
        Copy-Item (Join-Path $RootDir "USER_GUIDE.md") $PackageDir
    }

    if (Test-Path (Join-Path $RootDir "LICENSE-Apache-2.0.txt")) {
        Copy-Item (Join-Path $RootDir "LICENSE-Apache-2.0.txt") $PackageDir
    }

    @"
name=skillctrl
version=$Version
target=$Target
profile=$Profile
binaries=$($BinNames -join ',')
"@ | Set-Content -Path (Join-Path $PackageDir "BUILD_INFO.txt")

    if (Test-Path $ArchivePath) {
        Remove-Item $ArchivePath -Force
    }

    Compress-Archive -Path (Join-Path $PackageDir "*") -DestinationPath $ArchivePath -Force

    $Hash = (Get-FileHash $ArchivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    "$Hash  $(Split-Path -Leaf $ArchivePath)" | Set-Content -Path $ChecksumPath

    Write-Host "Package created:"
    Write-Host "  Archive: $ArchivePath"
    Write-Host "  Checksum: $ChecksumPath"
    foreach ($BinaryPath in $BinaryPaths) {
        Write-Host "  Binary: $BinaryPath"
    }
}
finally {
    if (Test-Path $WorkDir) {
        Remove-Item $WorkDir -Recurse -Force
    }
}
