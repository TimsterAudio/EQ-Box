# Shared by install-engine.ps1 and remove-engine.ps1.

$DllName      = 'eq_box_apo.dll'
$InstallDir   = Join-Path $env:ProgramFiles 'EQ Box'
$BackupDir    = Join-Path $InstallDir 'backup'
$SettingsDir  = Join-Path $env:ProgramData 'EQ Box'

# Never change: installed systems point at this CLSID.
$ClsidText    = '{0FEBC01A-A301-491F-BBCD-17284A13CB61}'
$IidApoText   = '{FD7F2B29-24D0-4B5C-B177-592C39F9CA10}'   # IAudioProcessingObject
$ModeDefault  = '{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}'   # AUDIO_SIGNALPROCESSINGMODE_DEFAULT

# FxProperties value names (property keys)
$PkeyEfx      = '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},7'  # PKEY_FX_EndpointEffectClsid
$PkeyEfxModes = '{d3993a3f-99c2-4402-b5ec-a92a0367664b},7'  # PKEY_EFX_ProcessingModes_Supported_For_Streaming

# Properties value names
$PkeyDeviceDesc    = '{a45c254e-df1c-4efd-8020-67d146a850e0},2'  # e.g. "Speakers"
$PkeyInterfaceName = '{b3f8fa53-0004-438e-9003-51a46e139bfc},6'  # e.g. "Realtek(R) Audio"
$PkeyDisableSysFx  = '{1da5d803-d492-4edd-8c23-e0c0ffee7f0e},5'  # Audio enhancements off

$RenderRoot = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render'

function Get-RenderDevices {
    foreach ($k in Get-ChildItem -LiteralPath $RenderRoot) {
        $guid = $k.PSChildName
        $state = $k.GetValue('DeviceState', 0)
        $props = Join-Path $k.PSPath 'Properties'
        $desc = $null; $iface = $null; $sysFxOff = $false
        if (Test-Path -LiteralPath $props) {
            $p = Get-Item -LiteralPath $props
            $desc = $p.GetValue($PkeyDeviceDesc, $null)
            $iface = $p.GetValue($PkeyInterfaceName, $null)
            $sysFxOff = ($p.GetValue($PkeyDisableSysFx, 0) -eq 1)
        }
        $fx = Join-Path $k.PSPath 'FxProperties'
        $efx = $null
        if (Test-Path -LiteralPath $fx) {
            $efx = (Get-Item -LiteralPath $fx).GetValue($PkeyEfx, $null)
        }
        $label = if ($desc -and $iface) { "$desc ($iface)" } elseif ($desc) { $desc } else { $guid }
        [pscustomobject]@{
            Guid            = $guid
            Id              = "{0.0.0.00000000}.$guid"
            Key             = "$RenderRoot\$guid"
            Label           = $label
            Active          = (($state -band 0x1) -ne 0)
            EnhancementsOff = $sysFxOff
            EndpointEffect  = $efx
            HasEqBox        = ($efx -eq $ClsidText)
        }
    }
}

# The device registry keys can only be changed by SYSTEM, so those steps
# run as a one-off scheduled task under the SYSTEM account. The script goes
# in the install folder, which only administrators can write to.
function Invoke-AsSystem([string]$Script) {
    $dir = Join-Path $InstallDir 'setup'
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $ps1  = Join-Path $dir 'system-step.ps1'
    $done = Join-Path $dir 'system-step.done'
    $fail = Join-Path $dir 'system-step.error'
    Remove-Item -LiteralPath $done, $fail -ErrorAction SilentlyContinue

    $wrapped = @"
try {
$Script
    Set-Content -LiteralPath '$done' -Value ok
} catch {
    Set-Content -LiteralPath '$fail' -Value (`$_ | Out-String)
}
"@
    Set-Content -LiteralPath $ps1 -Value $wrapped -Encoding UTF8

    $task = 'EQ Box setup step'
    $action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$ps1`""
    $principal = New-ScheduledTaskPrincipal -UserId 'S-1-5-18' -LogonType ServiceAccount -RunLevel Highest
    Register-ScheduledTask -TaskName $task -Action $action -Principal $principal -Force | Out-Null
    try {
        Start-ScheduledTask -TaskName $task
        $deadline = (Get-Date).AddSeconds(60)
        while (-not (Test-Path -LiteralPath $done) -and -not (Test-Path -LiteralPath $fail)) {
            if ((Get-Date) -gt $deadline) { throw 'The SYSTEM step did not finish within 60 seconds.' }
            Start-Sleep -Milliseconds 250
        }
    } finally {
        Unregister-ScheduledTask -TaskName $task -Confirm:$false -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $ps1 -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $fail) {
        $msg = Get-Content -LiteralPath $fail -Raw
        Remove-Item -LiteralPath $fail -ErrorAction SilentlyContinue
        throw "The SYSTEM step failed: $msg"
    }
    Remove-Item -LiteralPath $done -ErrorAction SilentlyContinue
}
