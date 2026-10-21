# 🛠️ Alexandria Developer & Distribution Scripts

Utility scripts for cross-platform installation, capsule integrity verification, and performance benchmarking.

## Script Index

| Script | Platform | Purpose |
| :--- | :--- | :--- |
| **[`install.sh`](install.sh)** | Linux / macOS | One-line installer using `cargo install` or pre-built release binary. |
| **[`install.ps1`](install.ps1)** | Windows PowerShell | Windows installation script with PATH registration. |
| **[`validate-capsules.sh`](validate-capsules.sh)** | POSIX Shell | Verifies SHA-256 digests of all 225 Master Capsules. |
| **[`validate-capsules.ps1`](validate-capsules.ps1)** | Windows PowerShell | Windows capsule integrity check. |
| **[`benchmark-latency.sh`](benchmark-latency.sh)** | POSIX Shell | Measures AST parse latency and SQLite WAL RRF query speed. |
