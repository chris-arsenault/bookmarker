#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$KeystorePath = if ([string]::IsNullOrWhiteSpace($env:LINKDROP_ANDROID_KEYSTORE)) {
    Join-Path (Join-Path (Join-Path $HOME ".android") "linkdrop") "linkdrop-release.jks"
} else {
    $env:LINKDROP_ANDROID_KEYSTORE
}
$KeyAlias = if ([string]::IsNullOrWhiteSpace($env:LINKDROP_ANDROID_KEY_ALIAS)) {
    "linkdrop"
} else {
    $env:LINKDROP_ANDROID_KEY_ALIAS
}
$DName = if ([string]::IsNullOrWhiteSpace($env:LINKDROP_ANDROID_DNAME)) {
    "CN=Linkdrop, OU=Linkdrop, O=Ahara, C=US"
} else {
    $env:LINKDROP_ANDROID_DNAME
}

if (-not (Get-Command keytool -ErrorAction SilentlyContinue)) {
    Write-Error "keytool not found. Install a JDK before creating the release keystore."
    exit 1
}

if (Test-Path -LiteralPath $KeystorePath) {
    Write-Output "Release keystore already exists:"
    Write-Output "  $KeystorePath"
    Write-Output "Refusing to overwrite it."
    exit 0
}

$KeystoreDir = Split-Path -Parent $KeystorePath
New-Item -ItemType Directory -Force -Path $KeystoreDir | Out-Null

Write-Output "Creating Linkdrop Android release keystore:"
Write-Output "  $KeystorePath"
Write-Output ""
Write-Output "Use a strong password and store it somewhere durable."
Write-Output "When keytool asks for the key password, press RETURN to reuse the keystore password."
Write-Output ""

& keytool -genkeypair -v `
    -keystore $KeystorePath `
    -alias $KeyAlias `
    -keyalg RSA `
    -keysize 4096 `
    -validity 10000 `
    -storetype PKCS12 `
    -dname $DName

if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

Write-Output ""
Write-Output "Created $KeystorePath"
Write-Output "Keep this file and its password. Losing it prevents future upgrades of the same app id."
