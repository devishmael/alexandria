# Alexandria Kernel Windows Installer
$ErrorActionPreference = "Stop"

Write-Host "🏛️ Installing Alexandria Kernel for Windows..." -ForegroundColor Cyan

if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Write-Host "⚙️ Found Cargo. Installing from source repository..." -ForegroundColor Yellow
    cargo install --git https://github.com/devishmael/alexandria.git
    Write-Host "✅ Alexandria installed successfully!" -ForegroundColor Green
    & alexandria -v
} else {
    Write-Error "Cargo was not found in PATH. Please install Rust from https://rustup.rs and rerun."
}
