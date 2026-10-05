# Packs runforge.exe into an unsigned MSIX for product 9PHL1HX0CGMF.
# Partner Center signs the upload. This script does not sign, and it refuses
# a package whose identity drifted or that vendors the trainer.
#Requires -Version 7
param(
    [string]$Exe,
    [string]$Out
)

$ErrorActionPreference = 'Stop'

$RepoRoot = Split-Path $PSScriptRoot -Parent
$Template = Join-Path $PSScriptRoot 'msix'
$Manifest = Join-Path $Template 'AppxManifest.xml'

$ExpectedName = 'mcp-tool-shop.RunForge-Desktop'
$ExpectedPublisher = 'CN=5305D976-6952-4F00-9C21-3A5DB090359F'
$ExpectedArch = 'x64'
$ExpectedExe = 'runforge.exe'
$ExpectedEntry = 'Windows.FullTrustApplication'
$ExpectedDisplayName = 'RunForge'
$ExpectedPublisherDisplay = 'mcp-tool-shop'
$ExpectedVersion = '2.0.0.0'
$VersionFloor = [version]'1.0.1.0'

function Find-MakeAppx {
    $roots = @(
        (Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'),
        (Join-Path $env:ProgramFiles 'Windows Kits\10\bin')
    )
    $hits = foreach ($root in $roots) {
        if (Test-Path $root) {
            Get-ChildItem $root -Recurse -Filter makeappx.exe -ErrorAction SilentlyContinue |
                Where-Object { $_.FullName -match '\\x64\\makeappx.exe$' }
        }
    }
    $best = $hits | Sort-Object FullName -Descending | Select-Object -First 1
    if (-not $best) {
        throw 'makeappx.exe (x64) was not found. Install the Windows 10 SDK.'
    }
    return $best.FullName
}

function Read-Identity([string]$Path) {
    [xml]$doc = Get-Content -Path $Path -Raw
    $ns = New-Object System.Xml.XmlNamespaceManager($doc.NameTable)
    $ns.AddNamespace('m', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10')
    $ns.AddNamespace('r', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities')
    $identity = $doc.SelectSingleNode('/m:Package/m:Identity', $ns)
    $props = $doc.SelectSingleNode('/m:Package/m:Properties', $ns)
    $app = $doc.SelectSingleNode('/m:Package/m:Applications/m:Application', $ns)
    $caps = $doc.SelectNodes('/m:Package/m:Capabilities/*', $ns)
    if (-not $identity -or -not $props -or -not $app) {
        throw "Manifest is missing Identity, Properties, or Application: $Path"
    }
    $capNames = @()
    foreach ($cap in $caps) { $capNames += [string]$cap.Name }
    return [pscustomobject]@{
        Name                 = [string]$identity.Name
        Publisher            = [string]$identity.Publisher
        Version              = [string]$identity.Version
        Arch                 = [string]$identity.ProcessorArchitecture
        Executable           = [string]$app.Executable
        EntryPoint           = [string]$app.EntryPoint
        DisplayName          = [string]$props.DisplayName
        PublisherDisplayName = [string]$props.PublisherDisplayName
        Capabilities         = $capNames
    }
}

function Assert-Identity($id, [string]$Where) {
    $problems = @()
    if ($id.Name -ne $ExpectedName) { $problems += "name is '$($id.Name)'" }
    if ($id.Publisher -ne $ExpectedPublisher) { $problems += 'publisher does not match the Store product' }
    if ($id.Arch -ne $ExpectedArch) { $problems += "architecture is '$($id.Arch)'" }
    if ($id.Executable -ne $ExpectedExe) { $problems += "executable is '$($id.Executable)'" }
    if ($id.EntryPoint -ne $ExpectedEntry) { $problems += "entry point is '$($id.EntryPoint)'" }
    if ($id.DisplayName -ne $ExpectedDisplayName) { $problems += "display name is '$($id.DisplayName)'" }
    if ($id.PublisherDisplayName -ne $ExpectedPublisherDisplay) { $problems += "publisher display name is '$($id.PublisherDisplayName)'" }
    if ($id.Version -ne $ExpectedVersion) { $problems += "version is '$($id.Version)'" }
    $parts = $id.Version.Split('.')
    if ($parts.Count -ne 4) {
        $problems += "version '$($id.Version)' is not four parts"
    } else {
        $parsed = [version]$id.Version
        if ($parsed -le $VersionFloor) { $problems += "version $($id.Version) is not above 1.0.1.0" }
        if ([int]$parts[3] -ne 0) { $problems += 'the fourth version part is not 0' }
    }
    $caps = @($id.Capabilities)
    if ($caps -notcontains 'runFullTrust') { $problems += 'runFullTrust is missing' }
    foreach ($cap in $caps) {
        if ($cap -ne 'runFullTrust') { $problems += "unexpected capability '$cap'" }
    }
    if ($problems.Count -gt 0) {
        throw ("Identity check failed ({0}): " -f $Where) + ($problems -join '; ')
    }
}

function Set-ReleaseRemap {
    # Stable rustc. trim-paths is still unstable on 1.98.1.
    # Prefixes are computed here and are not written into the package.
    $flags = New-Object System.Collections.Generic.List[string]
    function Add-Remap([string]$From, [string]$To) {
        if (-not $From) { return }
        $flags.Add("--remap-path-prefix=${From}=${To}")
        $forward = $From -replace '\\', '/'
        if ($forward -ne $From) { $flags.Add("--remap-path-prefix=${forward}=${To}") }
    }
    $cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
    $rustupHome = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { Join-Path $env:USERPROFILE '.rustup' }
    Add-Remap $cargoHome 'cargo-home'
    Add-Remap $rustupHome 'rustup'
    Add-Remap $RepoRoot 'runforge'
    Add-Remap $env:USERPROFILE 'home'
    $env:RUSTFLAGS = $flags -join ' '
}

function Find-Exe {
    Set-ReleaseRemap
    Push-Location $RepoRoot
    try {
        & cargo build --locked --release -p runforge
        if ($LASTEXITCODE -ne 0) { throw "release build failed with exit $LASTEXITCODE" }
    } finally {
        Pop-Location
        Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
    }
    $candidate = Join-Path $RepoRoot 'target\release\runforge.exe'
    if (Test-Path $candidate) { return $candidate }
    throw 'target\release\runforge.exe was not found after the release build.'
}

function Remove-FlaggedLicenseFiles([string]$Dir) {
    $scanner = Join-Path $env:USERPROFILE '.grok\bin\identity-scan.py'
    if (-not (Test-Path $scanner)) { throw 'Identity scanner is missing.' }
    $resolved = (Resolve-Path $Dir).Path.Replace('\', '/')
    $output = & python $scanner $Dir
    if ($LASTEXITCODE -eq 0) { return 0 }
    $removed = 0
    foreach ($line in @($output)) {
        if ($line -match '^HIT \S+ (.+):(\d+)\s*$') {
            $path = $Matches[1]
            if (-not $path.Replace('\', '/').StartsWith($resolved)) { continue }
            if (Test-Path -LiteralPath $path) {
                Remove-Item -LiteralPath $path -Force
                $removed++
            }
        }
    }
    if ($removed -eq 0) { throw 'Identity scan failed and no license file could be removed.' }
    return $removed
}

function Add-Notices([string]$Stage) {
    Push-Location $RepoRoot
    try {
        $json = & cargo metadata --locked --format-version 1 --filter-platform x86_64-pc-windows-msvc | Out-String
        if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    } finally {
        Pop-Location
    }
    $meta = $json | ConvertFrom-Json
    $dest = Join-Path $Stage 'licenses'
    New-Item -ItemType Directory -Path $dest | Out-Null
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add('Third-party notices for the RunForge package.')
    $lines.Add('Each entry is a crate name, version, and SPDX license.')
    $lines.Add('')
    $seen = @{}
    foreach ($pkg in $meta.packages) {
        if ($pkg.name -eq 'runforge' -or $pkg.name -eq 'runforge-core') { continue }
        $key = '{0} {1}' -f $pkg.name, $pkg.version
        if ($seen.ContainsKey($key)) { continue }
        $seen[$key] = $true
        $license = $pkg.license
        if (-not $license) { $license = 'NOASSERTION' }
        $lines.Add("$key — $license")
        $manifestDir = Split-Path $pkg.manifest_path -Parent
        $licenseFiles = Get-ChildItem -Path $manifestDir -File -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|UNLICENSE|NOTICE)' }
        foreach ($file in $licenseFiles) {
            $safe = ($key -replace '[^A-Za-z0-9._-]', '_')
            $target = Join-Path $dest ($safe + '-' + $file.Name)
            if (-not (Test-Path $target)) {
                Copy-Item $file.FullName $target
            }
        }
    }
    $removed = Remove-FlaggedLicenseFiles $dest
    if ($removed -eq 1) {
        $lines.Add('')
        $lines.Add('1 upstream license file was left out because it contained a contact address. The SPDX identifier above is the record of those terms.')
    } elseif ($removed -gt 1) {
        $lines.Add('')
        $lines.Add("$removed upstream license files were left out because they contained a contact address. The SPDX identifier above is the record of those terms.")
    }
    $notice = Join-Path $Stage 'NOTICE'
    [System.IO.File]::WriteAllLines($notice, $lines)
    Copy-Item (Join-Path $RepoRoot 'LICENSE') (Join-Path $Stage 'LICENSE')
}

function Assert-Layout([string[]]$Listing) {
    foreach ($entry in $Listing) {
        $normalized = $entry.Replace('\', '/')
        foreach ($part in $normalized.Split('/')) {
            if ($part -match '^(?i)(python(\d+)?(\.exe|\.dll)?|pytorch|torch(\.dll)?|bun(\.exe)?|backpropagate)$') {
                throw "Package contains a banned file name: $part"
            }
        }
    }
}

if (-not (Test-Path $Manifest)) { throw "Missing $Manifest" }

$sourceId = Read-Identity $Manifest
Assert-Identity $sourceId 'packaging/msix/AppxManifest.xml'

if (-not $Exe) { $Exe = Find-Exe }
if (-not (Test-Path $Exe)) { throw "Executable not found: $Exe" }
$exeItem = Get-Item $Exe
if ($exeItem.Length -lt 1MB) {
    throw "Executable is $($exeItem.Length) bytes. A release binary is larger than that."
}

if (-not $Out) {
    $releaseDir = Join-Path $RepoRoot 'release'
    New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null
    $Out = Join-Path $releaseDir 'RunForge_2.0.0.0_x64.msix'
}

$makeappx = Find-MakeAppx
$stage = Join-Path $env:TEMP ('runforge-msix-stage-' + [guid]::NewGuid().ToString('n'))
$check = Join-Path $env:TEMP ('runforge-msix-check-' + [guid]::NewGuid().ToString('n'))

try {
    New-Item -ItemType Directory -Path $stage | Out-Null
    Copy-Item -Path (Join-Path $Template '*') -Destination $stage -Recurse -Force
    Copy-Item -Path $Exe -Destination (Join-Path $stage $ExpectedExe) -Force
    Add-Notices $stage

    & $makeappx pack /o /d $stage /p $Out
    if ($LASTEXITCODE -ne 0) { throw "makeappx pack failed with exit $LASTEXITCODE" }

    $listing = @(& tar -tf $Out)
    if ($listing -match 'AppxSignature\.p7x') {
        throw 'Package contains AppxSignature.p7x. The Store upload must be unsigned.'
    }
    if ($listing -notcontains $ExpectedExe) {
        throw "Packed archive is missing $ExpectedExe"
    }
    Assert-Layout $listing

    New-Item -ItemType Directory -Path $check | Out-Null
    & tar -xf $Out -C $check AppxManifest.xml
    $packedId = Read-Identity (Join-Path $check 'AppxManifest.xml')
    Assert-Identity $packedId 'packed AppxManifest.xml'
}
finally {
    if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
    if (Test-Path $check) { Remove-Item -Recurse -Force $check }
}

$scanner = Join-Path $env:USERPROFILE '.grok\bin\identity-scan.py'
if (Test-Path $scanner) {
    & python $scanner $Out
    if ($LASTEXITCODE -ne 0) {
        Remove-Item -Force $Out
        throw 'Identity scan HIT. The package was deleted and must not be uploaded.'
    }
} else {
    Remove-Item -Force $Out
    throw 'Identity scan did not run. The package was deleted.'
}

Write-Output 'RESULT PASS'
Write-Output 'package release/RunForge_2.0.0.0_x64.msix'
Write-Output "name $($sourceId.Name)"
Write-Output "version $($sourceId.Version)"
Write-Output "arch $($sourceId.Arch)"
Write-Output "executable $($sourceId.Executable)"
Write-Output 'signed no'
