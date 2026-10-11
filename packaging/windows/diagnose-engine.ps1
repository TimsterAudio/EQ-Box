<#
.SYNOPSIS
    Shows what EQ Box's system-wide engine is doing. Changes nothing.

.DESCRIPTION
    Run from an Administrator PowerShell and send the output to the
    developer:
        powershell -ExecutionPolicy Bypass -File .\diagnose-engine.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot 'engine-common.ps1')

function Section([string]$title) { Write-Host ''; Write-Host "== $title" -ForegroundColor Cyan }

Section 'System'
Write-Host ("Windows: {0}" -f [Environment]::OSVersion.VersionString)
Write-Host ("64-bit PowerShell: {0}" -f [Environment]::Is64BitProcess)
Get-Service -Name audiosrv, AudioEndpointBuilder | ForEach-Object { Write-Host ("Service {0}: {1}" -f $_.Name, $_.Status) }
$dll = Join-Path $InstallDir $DllName
Write-Host ("Engine DLL: {0} ({1})" -f $dll, $(if (Test-Path -LiteralPath $dll) { (Get-Item -LiteralPath $dll).LastWriteTime } else { 'missing' }))
$com = "HKLM:\SOFTWARE\Classes\CLSID\$ClsidText\InprocServer32"
Write-Host ("COM registration: {0}" -f $(if (Test-Path -LiteralPath $com) { (Get-Item -LiteralPath $com).GetValue('') } else { 'missing' }))
$apo = "HKLM:\SOFTWARE\Classes\AudioEngine\AudioProcessingObjects\$ClsidText"
Write-Host ("Audio engine registration: {0}" -f $(if (Test-Path -LiteralPath $apo) { 'present' } else { 'missing' }))
$audioKey = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio'
Write-Host ("DisableProtectedAudioDG: {0}" -f (Get-ItemProperty -LiteralPath $audioKey -ErrorAction SilentlyContinue).DisableProtectedAudioDG)

Section 'Is the engine loaded by Windows audio?'
tasklist /m $DllName

Section 'Playback devices'
foreach ($d in Get-RenderDevices) {
    $mark = if ($d.HasEqBox) { '  <-- EQ Box' } else { '' }
    Write-Host ("{0}  state {1}  enhancements off: {2}  {3}{4}" -f $d.Guid, $d.State, $d.EnhancementsOff, $d.Label, $mark)
}

foreach ($d in Get-RenderDevices | Where-Object HasEqBox) {
    Section "Effect settings of: $($d.Label)"
    $fx = Get-Item -LiteralPath "$($d.Key)\FxProperties"
    foreach ($name in $fx.GetValueNames()) {
        $v = $fx.GetValue($name)
        if ($v -is [array]) { $v = $v -join ' | ' }
        Write-Host ("{0} = {1}" -f $name, $v)
    }
}

Section 'Settings file (written by the EQ Box window)'
$settings = Join-Path $SettingsDir 'engine.json'
if (Test-Path -LiteralPath $settings) {
    Write-Host ("Last written: {0}" -f (Get-Item -LiteralPath $settings).LastWriteTime)
    $j = Get-Content -LiteralPath $settings -Raw | ConvertFrom-Json
    $i = 0
    foreach ($b in $j.bands) { $i++; if ($b.enabled -and $b.gain_db -ne 0) { Write-Host ("  band {0}: {1:+0.0;-0.0} dB at {2} Hz" -f $i, $b.gain_db, $b.freq) } }
    Write-Host ("  preamp {0} dB, bypass {1}" -f $j.preamp_db, $j.bypass)
} else {
    Write-Host 'missing'
}
icacls $SettingsDir

Section 'Engine status (written by the engine itself)'
$status = Join-Path $SettingsDir 'engine-status.txt'
if (Test-Path -LiteralPath $status) {
    Write-Host ("Last written: {0} (now {1})" -f (Get-Item -LiteralPath $status).LastWriteTime, (Get-Date))
    Get-Content -LiteralPath $status
} else {
    Write-Host 'No status file: Windows has not run the engine since this build was installed.'
}
