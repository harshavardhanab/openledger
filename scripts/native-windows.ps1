# WebView2 150+ ignores driver environment overrides in elevated hosts.
# Run the test chain at medium integrity, as Tauri's own Windows CI does:
# https://github.com/tauri-apps/tauri/blob/dev/.github/workflows/test-api-e2e.yml
$ErrorActionPreference = 'Stop'
choco install gsudo --yes --no-progress
if ($LASTEXITCODE -ne 0) { throw 'Could not install gsudo' }

# Grant only the existing runner user access to its test workspace, never Everyone.
$sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
icacls $env:GITHUB_WORKSPACE /grant "*${sid}:(OI)(CI)M" /T /C /Q
if ($LASTEXITCODE -ne 0) { throw 'Could not grant the runner access to its workspace' }
if ($env:OPEN_LEDGER_BINARY) {
  $directory = Split-Path $env:OPEN_LEDGER_BINARY
  icacls $directory /grant "*${sid}:(OI)(CI)RX" /T /C /Q
  if ($LASTEXITCODE -ne 0) { throw 'Could not grant access to the installed executable' }
}
gsudo --integrity Medium node scripts/desktop-smoke.mjs
exit $LASTEXITCODE
