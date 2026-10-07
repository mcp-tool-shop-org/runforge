# Packs runforge.exe into an unsigned MSIX for product 9PHL1HX0CGMF.
# Partner Center signs the upload. This script does not sign, and it refuses
# a package whose identity drifted or that vendors the trainer.
# Logo names in the manifest are unqualified. makepri writes resources.pri
# so those names resolve to the scale-100 and scale-200 files. The config
# stays out of the package.
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
# The 1.0.x MAUI package used the template's Application Id. Keeping it means
# an update keeps the user's Start and taskbar pins.
$ExpectedAppId = 'App'
$ExpectedDisplayName = 'RunForge'
$ExpectedPublisherDisplay = 'mcp-tool-shop'
$ExpectedVersion = '2.0.0.0'
$VersionFloor = [version]'1.0.1.0'

function Find-SdkBin {
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
    return $best.DirectoryName
}

function Assert-TileSizes([string]$Assets) {
    Add-Type -AssemblyName System.Drawing
    $expected = @(
        @{ Name = 'StoreLogo.scale-100.png'; W = 50; H = 50 },
        @{ Name = 'StoreLogo.scale-200.png'; W = 100; H = 100 },
        @{ Name = 'Square44x44Logo.scale-100.png'; W = 44; H = 44 },
        @{ Name = 'Square44x44Logo.scale-200.png'; W = 88; H = 88 },
        @{ Name = 'Square71x71Logo.scale-100.png'; W = 71; H = 71 },
        @{ Name = 'Square71x71Logo.scale-200.png'; W = 142; H = 142 },
        @{ Name = 'Square150x150Logo.scale-100.png'; W = 150; H = 150 },
        @{ Name = 'Square150x150Logo.scale-200.png'; W = 300; H = 300 },
        @{ Name = 'Wide310x150Logo.scale-100.png'; W = 310; H = 150 },
        @{ Name = 'Wide310x150Logo.scale-200.png'; W = 620; H = 300 },
        @{ Name = 'Square310x310Logo.scale-100.png'; W = 310; H = 310 },
        @{ Name = 'Square310x310Logo.scale-200.png'; W = 620; H = 620 },
        @{ Name = 'SplashScreen.scale-100.png'; W = 620; H = 300 },
        @{ Name = 'SplashScreen.scale-200.png'; W = 1240; H = 600 }
    )
    foreach ($tile in $expected) {
        $path = Join-Path $Assets $tile.Name
        if (-not (Test-Path $path)) { throw "Missing tile $($tile.Name)" }
        $img = [System.Drawing.Image]::FromFile((Resolve-Path $path))
        try {
            if ($img.Width -ne $tile.W -or $img.Height -ne $tile.H) {
                throw "$($tile.Name) is $($img.Width)x$($img.Height); expected $($tile.W)x$($tile.H)"
            }
        } finally {
            $img.Dispose()
        }
    }
}

function Get-PeSubsystem([string]$Path) {
    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $reader = New-Object System.IO.BinaryReader($stream)
        $stream.Position = 0x3C
        $pe = $reader.ReadInt32()
        if ($pe -le 0) { throw "PE header offset is invalid in $Path" }
        $stream.Position = $pe
        $signature = $reader.ReadUInt32()
        if ($signature -ne 0x00004550) { throw "PE signature is missing in $Path" }
        $stream.Position = $pe + 24
        $magic = $reader.ReadUInt16()
        if ($magic -ne 0x20B) { throw "Expected a PE32+ binary, found magic $magic" }
        $stream.Position = $pe + 24 + 68
        return $reader.ReadUInt16()
    } finally {
        $stream.Dispose()
    }
}

function Assert-StoreBinary([string]$Path, [string]$Mt) {
    $subsystem = Get-PeSubsystem $Path
    if ($subsystem -ne 2) {
        throw "runforge.exe subsystem is $subsystem. The Store build must be a window (2), not a console (3)."
    }
    $extracted = Join-Path $env:TEMP ('runforge-embedded-' + [guid]::NewGuid().ToString('n') + '.manifest')
    try {
        & $Mt -nologo "-inputresource:${Path};#1" "-out:$extracted"
        if ($LASTEXITCODE -ne 0) { throw "mt.exe could not read the embedded manifest (exit $LASTEXITCODE)" }
        if (-not (Test-Path $extracted)) { throw 'mt.exe did not write the embedded manifest' }
        $text = Get-Content -Path $extracted -Raw
        if ($text -notmatch 'PerMonitorV2') { throw 'Embedded manifest is missing PerMonitorV2 DPI awareness.' }
        if ($text -notmatch 'dpiAware') { throw 'Embedded manifest is missing dpiAware.' }
        if ($text -notmatch 'asInvoker') { throw 'Embedded manifest is missing asInvoker.' }
    } finally {
        if (Test-Path $extracted) { Remove-Item -Force $extracted }
    }
}

function Add-ResourceIndex([string]$Stage, [string]$MakePri) {
    # The config is a build input. It is written outside the stage so the
    # package cannot pick it up.
    $work = Join-Path $env:TEMP ('runforge-pri-' + [guid]::NewGuid().ToString('n'))
    New-Item -ItemType Directory -Path $work | Out-Null
    $config = Join-Path $work 'priconfig.xml'
    try {
        & $MakePri createconfig /cf $config /dq en-US /o
        if ($LASTEXITCODE -ne 0) { throw "makepri createconfig failed with exit $LASTEXITCODE" }
        [xml]$doc = Get-Content -Path $config -Raw
        $index = $doc.SelectSingleNode('/resources/index')
        if (-not $index) { throw 'makepri config has no index element.' }
        $folder = $index.SelectSingleNode('indexer-config[@type="folder"]')
        if (-not $folder) { throw 'makepri config has no folder indexer.' }
        # License filenames contain dots. Indexing them makes makepri treat
        # those dots as qualifiers. The index stays at the package root so
        # the resource names keep the Assets folder the manifest names.
        foreach ($exclude in @(
                @{ Type = 'extension'; Value = '.exe' },
                @{ Type = 'extension'; Value = '.pri' },
                @{ Type = 'extension'; Value = '.xml' },
                @{ Type = 'path'; Value = '\licenses' },
                @{ Type = 'path'; Value = '\NOTICE' },
                @{ Type = 'path'; Value = '\LICENSE' }
            )) {
            $node = $doc.CreateElement('exclude')
            $node.SetAttribute('type', $exclude.Type)
            $node.SetAttribute('value', $exclude.Value)
            $node.SetAttribute('doNotIndex', 'true')
            $node.SetAttribute('doNotTraverse', 'true')
            [void]$folder.AppendChild($node)
        }
        $scoped = Join-Path $work 'priconfig-assets.xml'
        $doc.Save($scoped)
        $pri = Join-Path $Stage 'resources.pri'
        $manifest = Join-Path $Stage 'AppxManifest.xml'
        $lines = @(& $MakePri new /pr $Stage /cf $scoped /mn $manifest /of $pri /o 2>&1 | ForEach-Object { "$_" })
        $lines | ForEach-Object { Write-Output $_ }
        if ($LASTEXITCODE -ne 0) { throw "makepri new failed with exit $LASTEXITCODE" }
        # makepri also writes straight to the console, so the count is read
        # from the resource index itself. Seven named resources, seven candidates.
        $dump = Join-Path $work 'resources.pri.xml'
        & $MakePri dump /if $pri /of $dump | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "makepri dump failed with exit $LASTEXITCODE" }
        [xml]$priXml = Get-Content -Path $dump -Raw
        $resources = @($priXml.SelectNodes('//NamedResource'))
        $expectedNames = @(
            'SplashScreen.png',
            'Square150x150Logo.png',
            'Square310x310Logo.png',
            'Square44x44Logo.png',
            'Square71x71Logo.png',
            'StoreLogo.png',
            'Wide310x150Logo.png'
        )
        $names = @($resources | ForEach-Object { $_.name })
        $missing = @($expectedNames | Where-Object { $names -notcontains $_ })
        $extra = @($names | Where-Object { $expectedNames -notcontains $_ })
        $badUri = @($resources | Where-Object { $_.uri -notmatch '/Files/Assets/' })
        if ($missing.Count -gt 0 -or $extra.Count -gt 0 -or $badUri.Count -gt 0) {
            throw "Resource index names drifted. Missing: $($missing -join ', '). Extra: $($extra -join ', ')."
        }
        if (-not (Test-Path $pri)) { throw 'makepri did not write resources.pri' }
        $scale200 = Join-Path $Stage 'resources.scale-200.pri'
        if (-not (Test-Path $scale200)) { throw 'makepri did not write resources.scale-200.pri' }
        $leaked = Join-Path $Stage 'priconfig.xml'
        if (Test-Path $leaked) { Remove-Item -Force $leaked }
    } finally {
        if (Test-Path $work) { Remove-Item -Recurse -Force $work }
    }
}

function Test-Listed([string[]]$Listing, [string]$Name) {
    foreach ($entry in $Listing) {
        $normalized = ($entry -replace '\\', '/').Trim()
        if ($normalized -eq $Name -or $normalized.EndsWith("/$Name")) { return $true }
    }
    return $false
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
        AppId                = [string]$app.Id
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
    if ($id.AppId -ne $ExpectedAppId) { $problems += "application id is '$($id.AppId)', and 1.0.x pins need '$ExpectedAppId'" }
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
        if ($pkg.name -in @('runforge', 'runforge-core', 'workbench')) { continue }
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
Assert-TileSizes (Join-Path $Template 'Assets')

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

$sdkBin = Find-SdkBin
$makeappx = Join-Path $sdkBin 'makeappx.exe'
$makepri = Join-Path $sdkBin 'makepri.exe'
$mt = Join-Path $sdkBin 'mt.exe'
foreach ($tool in @($makeappx, $makepri, $mt)) {
    if (-not (Test-Path $tool)) { throw "SDK tool is missing: $tool" }
}
$stage = Join-Path $env:TEMP ('runforge-msix-stage-' + [guid]::NewGuid().ToString('n'))
$check = Join-Path $env:TEMP ('runforge-msix-check-' + [guid]::NewGuid().ToString('n'))

$packed = $false
try {
    New-Item -ItemType Directory -Path $stage | Out-Null
    Copy-Item -Path (Join-Path $Template '*') -Destination $stage -Recurse -Force
    Copy-Item -Path $Exe -Destination (Join-Path $stage $ExpectedExe) -Force
    Add-Notices $stage
    Assert-StoreBinary (Join-Path $stage $ExpectedExe) $mt
    Add-ResourceIndex $stage $makepri

    if (Test-Path $Out) { Remove-Item -Force $Out }
    & $makeappx pack /o /d $stage /p $Out
    if ($LASTEXITCODE -ne 0) {
        if (Test-Path $Out) { Remove-Item -Force $Out }
        throw "makeappx pack failed with exit $LASTEXITCODE"
    }
    $packed = $true

    $listing = @(& tar -tf $Out)
    if ($listing -match 'AppxSignature\.p7x') {
        throw 'Package contains AppxSignature.p7x. The Store upload must be unsigned.'
    }
    if (-not (Test-Listed $listing $ExpectedExe)) {
        throw "Packed archive is missing $ExpectedExe"
    }
    foreach ($required in @('resources.pri', 'resources.scale-200.pri')) {
        if (-not (Test-Listed $listing $required)) {
            throw "Packed archive is missing $required"
        }
    }
    if ($listing -match '(?i)priconfig\.xml') {
        throw 'Package contains priconfig.xml. That file is a build input, not a payload.'
    }
    Assert-Layout $listing

    New-Item -ItemType Directory -Path $check | Out-Null
    & tar -xf $Out -C $check AppxManifest.xml
    $packedId = Read-Identity (Join-Path $check 'AppxManifest.xml')
    Assert-Identity $packedId 'packed AppxManifest.xml'
} catch {
    if ($packed -and (Test-Path $Out)) { Remove-Item -Force $Out }
    throw
} finally {
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

# 1.0.1 shipped as a bundle, and Partner Center refuses a plain package once a
# product has released one. Wrap the checked package in an unsigned bundle at the
# same version; the bundle is what gets uploaded.
$Bundle = [System.IO.Path]::ChangeExtension($Out, '.msixbundle')
$bundleDir = Join-Path $env:TEMP ('runforge-msix-bundle-' + [guid]::NewGuid().ToString('n'))
try {
    New-Item -ItemType Directory -Path $bundleDir | Out-Null
    Copy-Item -Path $Out -Destination $bundleDir
    if (Test-Path $Bundle) { Remove-Item -Force $Bundle }
    & $makeappx bundle /o /d $bundleDir /bv $ExpectedVersion /p $Bundle
    if ($LASTEXITCODE -ne 0) {
        if (Test-Path $Bundle) { Remove-Item -Force $Bundle }
        throw "makeappx bundle failed with exit $LASTEXITCODE"
    }
    $bundled = @(& tar -tf $Bundle)
    if (-not ($bundled -match [regex]::Escape([System.IO.Path]::GetFileName($Out)))) {
        throw 'The bundle does not contain the checked package.'
    }
    if ($bundled -match 'AppxSignature\.p7x') {
        throw 'Bundle contains AppxSignature.p7x. The Store upload must be unsigned.'
    }
} finally {
    if (Test-Path $bundleDir) { Remove-Item -Recurse -Force $bundleDir }
}
& python $scanner $Bundle
if ($LASTEXITCODE -ne 0) {
    Remove-Item -Force $Bundle
    throw 'Identity scan HIT on the bundle. It was deleted and must not be uploaded.'
}

Write-Output 'RESULT PASS'
Write-Output 'package release/RunForge_2.0.0.0_x64.msix'
Write-Output 'bundle release/RunForge_2.0.0.0_x64.msixbundle (upload this one)'
Write-Output "name $($sourceId.Name)"
Write-Output "version $($sourceId.Version)"
Write-Output "arch $($sourceId.Arch)"
Write-Output "executable $($sourceId.Executable)"
Write-Output 'signed no'
