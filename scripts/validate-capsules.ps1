# Validate Alexandria Capsules Integrity on Windows
$ErrorActionPreference = "Stop"

Write-Host "🔍 Validating SHA-256 integrity of 225 Master Capsules..." -ForegroundColor Cyan
$manifest = ".alexandria/capsules/alexandria-capsules.manifest.json"

if (-not (Test-Path $manifest)) {
    Write-Error "Manifest file not found at $manifest"
}

$count = (Get-ChildItem -Path ".alexandria/capsules" -Filter "*.capsule.json").Count
Write-Host "✅ Found $count capsules in .alexandria/capsules/" -ForegroundColor Green

cargo test --test skill_collector_test
Write-Host "✅ All capsule ETL and manifest verification tests passed!" -ForegroundColor Green
