# One-time setup script for the FBR Marble Tools Windows server.
# Run as Administrator in PowerShell.
#
# Prerequisites to complete before running this script:
#   1. NSSM:  winget install NSSM.NSSM
#   2. Rust:  winget install Rustlang.Rustup
#             After install, open a new shell and run:
#             rustup target add wasm32-unknown-unknown
#   3. cargo-leptos: cargo install cargo-leptos --locked
#   4. Generate a GitHub Actions runner registration token:
#      Go to your repo -> Settings -> Actions -> Runners -> New self-hosted runner
#      Copy the token shown — you will need it below.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$deployDirectory = 'C:\fbrmarble-tools'
$runnerDirectory = 'C:\actions-runner'
$githubRepoUrl   = 'https://github.com/YOUR_ORG/YOUR_REPO'
$runnerToken     = 'PASTE_TOKEN_HERE'

# --- App directories ---

New-Item -ItemType Directory -Force -Path $deployDirectory
New-Item -ItemType Directory -Force -Path "$deployDirectory\logs"

# --- NSSM service ---

nssm install fbrmarble-tools "$deployDirectory\server.exe"
nssm set fbrmarble-tools AppDirectory    $deployDirectory
nssm set fbrmarble-tools DisplayName     'FBR Marble Tools'
nssm set fbrmarble-tools Description     'FBR Marble Tools web server'
nssm set fbrmarble-tools Start           SERVICE_AUTO_START
nssm set fbrmarble-tools AppRestartDelay 5000
nssm set fbrmarble-tools AppStdout       "$deployDirectory\logs\stdout.log"
nssm set fbrmarble-tools AppStderr       "$deployDirectory\logs\stderr.log"
nssm set fbrmarble-tools AppRotateFiles  1
nssm set fbrmarble-tools AppRotateBytes  10485760

# --- GitHub Actions runner ---

New-Item -ItemType Directory -Force -Path $runnerDirectory

$runnerZip = "$env:TEMP\actions-runner.zip"
Invoke-WebRequest -Uri 'https://github.com/actions/runner/releases/download/v2.323.0/actions-runner-win-x64-2.323.0.zip' `
    -OutFile $runnerZip
Expand-Archive -Path $runnerZip -DestinationPath $runnerDirectory -Force
Remove-Item $runnerZip

Set-Location $runnerDirectory
.\config.cmd --url $githubRepoUrl --token $runnerToken --name 'fbr-windows-server' --runnergroup Default --labels self-hosted,Windows --work '_work' --unattended
.\svc.cmd install
.\svc.cmd start

Write-Host ""
Write-Host "Done. Next steps:"
Write-Host "  1. Add these GitHub Secrets: SAP_DB_HOST, SAP_DB_PORT, SAP_DB_USER, SAP_DB_PASSWORD, SAP_DB_NAME"
Write-Host "  2. Update the githubRepoUrl variable in this script before running (if you haven't already)."
Write-Host "  3. Push to main to trigger the first deploy."
Write-Host ""
Write-Host "Do not start the fbrmarble-tools service manually — it will fail until the first deploy copies server.exe."
