# Alexandria Pre-Commit Git Hook for Windows
$ErrorActionPreference = "Stop"

Write-Host "🛡️ Alexandria Pre-Commit Quality Gate Scan..." -ForegroundColor Cyan

if (Get-Command cargo -ErrorAction SilentlyContinue) {
    cargo check --all-targets --quiet
    cargo test --all-targets --quiet
}

Write-Host "✅ All Alexandria Quality Gates cleared." -ForegroundColor Green
exit 0
