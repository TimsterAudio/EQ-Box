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

# A 32-bit PowerShell sees a redirected copy of the registry, so rerun this
# script in the 64-bit one.
if ([Environment]::Is64BitOperatingSystem -and -not [Environment]::Is64BitProcess) {
    $ps64 = Join-Path $env:windir 'sysnative\WindowsPowerShell\v1.0\powershell.exe'
    & $ps64 -NoProfile -ExecutionPolicy Bypass -File $PSCommandPath
    exit $LASTEXITCODE
}
. (Join-Path $PSScriptRoot 'engine-common.ps1')
Exit-IfTawEqualiser 'remove'

Write-Host ''
Write-Host 'EQ Box: remove the system-wide engine' -ForegroundColor Cyan
Write-Host ''

$backups = @()
if (Test-Path -LiteralPath $BackupDir) {
    $backups = @(Get-ChildItem -LiteralPath $BackupDir -Filter '*.json' |
        Where-Object { $_.Name -ne 'global.json' -and $_.Name -notlike '*.acl.json' })
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

# Each backed-up device: the values first, then the key's original owner and
# permissions (which takes away the access setup gave Administrators).
foreach ($file in $backups) {
    $saved = Get-Content -LiteralPath $file.FullName -Raw | ConvertFrom-Json
    $fx = "$RenderRoot\$($file.BaseName)\FxProperties"
    # Only touch the values if EQ Box is actually on the device (an install
    # that stopped early never changed them).
    $ours = (Test-Path -LiteralPath $fx) -and ((Get-Item -LiteralPath $fx).GetValue($PkeyEfx, $null) -eq $ClsidText)
    if ($ours) {
        if (-not $saved.FxExisted) {
            Remove-Item -LiteralPath $fx -Recurse -Force
        } else {
            foreach ($name in @($PkeyEfx, $PkeyEfxModes, $PkeyChained)) {
                Remove-ItemProperty -LiteralPath $fx -Name $name -ErrorAction SilentlyContinue
            }
            foreach ($p in $saved.Values.PSObject.Properties) {
                $data = $p.Value.Data
                if ($p.Value.Kind -eq 'MultiString') { $data = [string[]]@($data) }
                New-ItemProperty -Force -LiteralPath $fx -Name $p.Name -Value $data -PropertyType $p.Value.Kind | Out-Null
            }
        }
    }
    $aclFile = Join-Path $BackupDir "$($file.BaseName).acl.json"
    if (Test-Path -LiteralPath $aclFile) {
        $acl = Get-Content -LiteralPath $aclFile -Raw | ConvertFrom-Json
        if (Test-Path -LiteralPath "HKLM:\$($acl.Key)") {
            Restore-KeySddl $acl.Key $acl.Sddl
        }
    }
}

# Devices with EQ Box but no backup (shouldn't happen) just lose EQ Box.
foreach ($o in $orphans) {
    Remove-ItemProperty -LiteralPath "$($o.Key)\FxProperties" -Name $PkeyEfx -ErrorAction SilentlyContinue
    Remove-ItemProperty -LiteralPath "$($o.Key)\FxProperties" -Name $PkeyChained -ErrorAction SilentlyContinue
}

# The protected-audio setting, as it was before
$global = Join-Path $BackupDir 'global.json'
if (Test-Path -LiteralPath $global) {
    $prev = (Get-Content -LiteralPath $global -Raw | ConvertFrom-Json).DisableProtectedAudioDG
    $audio = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio'
    if ($null -eq $prev) {
        Remove-ItemProperty -Path $audio -Name DisableProtectedAudioDG -ErrorAction SilentlyContinue
    } else {
        New-ItemProperty -Force -Path $audio -Name DisableProtectedAudioDG -Value ([int]$prev) -PropertyType DWord | Out-Null
    }
}

# Registrations
if ([System.Diagnostics.EventLog]::SourceExists('EQ Box')) {
    Remove-EventLog -Source 'EQ Box' -ErrorAction SilentlyContinue
}
Remove-Item -LiteralPath "HKLM:\SOFTWARE\Classes\AudioEngine\AudioProcessingObjects\$ClsidText" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath "HKLM:\SOFTWARE\Classes\CLSID\$ClsidText" -Recurse -Force -ErrorAction SilentlyContinue

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
