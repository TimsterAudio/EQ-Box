EQ Box - system-wide engine for Windows (TEST BUILD)
====================================================

This folder installs EQ Box's engine inside Windows' own audio engine, so
EQ Box equalises everything an output plays. No virtual cable is needed.
It's an early test build: try it on one device first.

INSTALL
  1. Right-click Start > "Terminal (Admin)" or "Windows PowerShell (Admin)".
  2. Go to this folder, for example:
       cd "$env:USERPROFILE\Downloads\eq-box-0.1.0-windows-x86_64\system-wide-engine"
  3. Run:
       powershell -ExecutionPolicy Bypass -File .\install-engine.ps1
  4. Pick your output from the list. Windows audio restarts (a second of
     silence).

USE
  Open eq-box.exe, set System to "Windows (system-wide)", and press
  Run / Stop. Play music in any app on that output and move the EQ.
  The EQ stays set after EQ Box closes. Use Bypass to switch it off.

REMOVE
  From an Administrator PowerShell in this folder:
       powershell -ExecutionPolicy Bypass -File .\remove-engine.ps1
  Every device goes back exactly as it was.

IF A DEVICE GOES SILENT
  Settings > System > Sound > (your device) > Audio enhancements: Off
  bypasses EQ Box straight away. Then run remove-engine.ps1.

WON'T AFFECT
  Apps using exclusive mode or ASIO, and devices with Audio enhancements
  turned off.
