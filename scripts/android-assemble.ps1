#Requires -Version 5.1
param(
    [ValidateSet("Debug", "Release")]
    [string] $BuildVariant = "Debug"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = (Resolve-Path (Join-Path $ScriptDir "..")).Path

function Get-AndroidSdkRoot {
    if (-not [string]::IsNullOrWhiteSpace($env:ANDROID_HOME)) {
        return $env:ANDROID_HOME
    }
    if (-not [string]::IsNullOrWhiteSpace($env:ANDROID_SDK_ROOT)) {
        return $env:ANDROID_SDK_ROOT
    }
    return Join-Path $HOME "android-sdk"
}

$SdkRoot = Get-AndroidSdkRoot
$PlatformPath = Join-Path (Join-Path $SdkRoot "platforms") "android-36"
if (-not (Test-Path -LiteralPath $PlatformPath -PathType Container)) {
    Write-Error "Android SDK platform android-36 not found. Set ANDROID_HOME or ANDROID_SDK_ROOT, or install it under $HOME/android-sdk."
    exit 1
}

$IsWindowsHost = [System.IO.Path]::DirectorySeparatorChar -eq "\"
$GradleWrapperName = if ($IsWindowsHost) { "gradlew.bat" } else { "gradlew" }
$GradleWrapper = Join-Path (Join-Path $RootDir "android") $GradleWrapperName
if (-not (Test-Path -LiteralPath $GradleWrapper -PathType Leaf)) {
    Write-Error "Gradle wrapper not found at $GradleWrapper."
    exit 1
}

$env:ANDROID_HOME = $SdkRoot
$env:ANDROID_SDK_ROOT = $SdkRoot

Push-Location $RootDir
try {
    & $GradleWrapper -p android --no-daemon ":app:assemble$BuildVariant"
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
} finally {
    Pop-Location
}
