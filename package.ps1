# Windows Packaging Script for Netool

$ErrorActionPreference = "Stop"

Write-Host "Building netool..."
cargo build --release

$distDir = "dist"
if (Test-Path $distDir) {
    Remove-Item -Path $distDir -Recurse -Force
}
New-Item -ItemType Directory -Path $distDir | Out-Null

Write-Host "Copying binaries..."
Copy-Item "target/release/netool-server.exe" -Destination $distDir
Copy-Item "target/release/netool-client.exe" -Destination $distDir

Write-Host "Copying static resources..."
if (Test-Path "static") {
    Copy-Item "static" -Destination $distDir -Recurse
} else {
    Write-Warning "Static directory not found! Web mode might not work."
}

Write-Host "Packaging complete. Output directory: $distDir"
Write-Host "Contents:"
Get-ChildItem $distDir
