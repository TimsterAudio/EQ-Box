<#
.SYNOPSIS
    Removes EQ Box's system-wide engine and puts every device back as it was.

.DESCRIPTION
    Restores each device's effect settings from the backups install-engine.ps1
    made, unregisters the engine, deletes the DLL and restarts Windows audio.
    The EQ Box app itself is untouched.

    Run from an Administrator PowerShell:
        powershell -ExecutionPolicy Bypass -File .\remove-engine.ps1
#>
#Requires -RunAsAdministrator
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'engine-common.ps1')

Write-Host ''
Write-Host 'EQ Box: remove the system-wide engine' -ForegroundColor Cyan
Write-Host ''

$backups = @()
if (Test-Path -LiteralPath $BackupDir) {
    $backups = @(Get-ChildItem -LiteralPath $BackupDir -Filter '*.json' | Where-Object Name -ne 'global.json')
}
foreach ($b in $backups) {
    $label = (Get-Content -LiteralPath $b.FullName -Raw | ConvertFrom-Json).Label
    Write-Host "Restoring: $label"
}

# Devices with EQ Box but no backup (shouldn't happen) just lose EQ Box.
$orphans = @(Get-RenderDevices | Where-Object HasEqBox | Where-Object {
    -not (Test-Path -LiteralPath (Join-Path $BackupDir "$($_.Guid).json"))
})
foreach ($o in $orphans) { Write-Host "Removing EQ Box from: $($o.Label)" }
$orphanKeys = ($orphans | ForEach-Object { "'$($_.Key)\FxProperties'" }) -join ', '
if (-not $orphanKeys) { $orphanKeys = '' }

Invoke-AsSystem @"
`$ErrorActionPreference = 'Stop'
`$render = '$RenderRoot'

# Each backed-up device
foreach (`$file in @(Get-ChildItem -LiteralPath '$BackupDir' -Filter '*.json' -ErrorAction SilentlyContinue | Where-Object Name -ne 'global.json')) {
    `$saved = Get-Content -LiteralPath `$file.FullName -Raw | ConvertFrom-Json
    `$fx = "`$render\`$(`$file.BaseName)\FxProperties"
    if (-not (Test-Path -LiteralPath `$fx)) { continue }
    if (-not `$saved.FxExisted) {
        Remove-Item -LiteralPath `$fx -Recurse -Force
        continue
    }
    foreach (`$name in @('$PkeyEfx', '$PkeyEfxModes')) {
        Remove-ItemProperty -LiteralPath `$fx -Name `$name -ErrorAction SilentlyContinue
    }
    foreach (`$p in `$saved.Values.PSObject.Properties) {
        `$kind = `$p.Value.Kind
        `$data = `$p.Value.Data
        if (`$kind -eq 'MultiString') { `$data = [string[]]@(`$data) }
        New-ItemProperty -Force -LiteralPath `$fx -Name `$p.Name -Value `$data -PropertyType `$kind | Out-Null
    }
}

# Devices with EQ Box but no backup
foreach (`$fx in @($orphanKeys)) {
    Remove-ItemProperty -LiteralPath `$fx -Name '$PkeyEfx' -ErrorAction SilentlyContinue
}

# The protected-audio setting, as it was before
`$global = Join-Path '$BackupDir' 'global.json'
if (Test-Path -LiteralPath `$global) {
    `$prev = (Get-Content -LiteralPath `$global -Raw | ConvertFrom-Json).DisableProtectedAudioDG
    `$audio = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio'
    if (`$null -eq `$prev) {
        Remove-ItemProperty -Path `$audio -Name DisableProtectedAudioDG -ErrorAction SilentlyContinue
    } else {
        New-ItemProperty -Force -Path `$audio -Name DisableProtectedAudioDG -Value ([int]`$prev) -PropertyType DWord | Out-Null
    }
}

# Registrations
Remove-Item -LiteralPath 'HKLM:\SOFTWARE\Classes\AudioEngine\AudioProcessingObjects\$ClsidText' -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath 'HKLM:\SOFTWARE\Classes\CLSID\$ClsidText' -Recurse -Force -ErrorAction SilentlyContinue
"@

Write-Host 'Restarting Windows audio...'
Stop-Service -Name audiosrv -Force
try {
    Remove-Item -LiteralPath (Join-Path $InstallDir $DllName) -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $BackupDir -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath (Join-Path $InstallDir 'setup') -Recurse -Force -ErrorAction SilentlyContinue
    if (-not (Get-ChildItem -LiteralPath $InstallDir -ErrorAction SilentlyContinue)) {
        Remove-Item -LiteralPath $InstallDir -Force -ErrorAction SilentlyContinue
    }
} finally {
    Start-Service -Name audiosrv
}

Write-Host ''
Write-Host 'Done. EQ Box is removed and every device is back as it was.' -ForegroundColor Green
Write-Host "(EQ Box's settings file in $SettingsDir is kept; delete it if you like.)"
