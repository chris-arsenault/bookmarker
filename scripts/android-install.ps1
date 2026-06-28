#Requires -Version 5.1
param(
    [ValidateSet("debug", "release")]
    [string] $Mode = "debug"
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

function Get-SingleApk {
    param(
        [string] $Directory,
        [string] $Filter,
        [string] $Description
    )

    $Apks = @(Get-ChildItem -LiteralPath $Directory -Filter $Filter -File -ErrorAction SilentlyContinue | Sort-Object FullName)
    if ($Apks.Count -ne 1) {
        Write-Error "Expected exactly one $Description APK in $Directory, found $($Apks.Count).`n$($Apks.FullName -join "`n")"
        exit 1
    }
    return $Apks[0].FullName
}

$SdkRoot = Get-AndroidSdkRoot
$Adb = Join-Path (Join-Path $SdkRoot "platform-tools") "adb.exe"
if (-not (Test-Path -LiteralPath $Adb -PathType Leaf)) {
    $Adb = Join-Path (Join-Path $SdkRoot "platform-tools") "adb"
}
if (-not (Test-Path -LiteralPath $Adb -PathType Leaf)) {
    Write-Error "adb not found under $SdkRoot/platform-tools. Set ANDROID_HOME or ANDROID_SDK_ROOT."
    exit 1
}

Push-Location $RootDir
try {
    if ($Mode -eq "debug") {
        & (Join-Path $ScriptDir "android-assemble.ps1") -BuildVariant Debug
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
        $Apk = Join-Path $RootDir "android/app/build/outputs/apk/debug/linkdrop-debug-v0.1.0-1.apk"
    } else {
        $ReleaseDir = Join-Path $RootDir "android/app/build/outputs/apk/release"
        $SignedApks = @(Get-ChildItem -LiteralPath $ReleaseDir -Filter "linkdrop-release-signed-*.apk" -File -ErrorAction SilentlyContinue | Sort-Object FullName)
        if ($SignedApks.Count -eq 0) {
            Write-Output "No signed release APK found. Running scripts/android-sign-release.ps1 first."
            & (Join-Path $ScriptDir "android-sign-release.ps1")
            if ($LASTEXITCODE -ne 0) {
                exit $LASTEXITCODE
            }
        }
        $Apk = Get-SingleApk -Directory $ReleaseDir -Filter "linkdrop-release-signed-*.apk" -Description "signed release"
    }

    if ($Apk -like "*unsigned*") {
        Write-Error "Refusing to install an unsigned release APK: $Apk"
        exit 1
    }
    if (-not (Test-Path -LiteralPath $Apk -PathType Leaf)) {
        Write-Error "APK not found: $Apk"
        exit 1
    }

    $AdbTarget = @()
    if (-not [string]::IsNullOrWhiteSpace($env:LINKDROP_ANDROID_SERIAL)) {
        $AdbTarget = @("-s", $env:LINKDROP_ANDROID_SERIAL)
    } else {
        $DeviceOutput = & $Adb devices
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
        $Devices = @($DeviceOutput | ForEach-Object {
            if ($_ -match "^(\S+)\s+device$") {
                $Matches[1]
            }
        })
        if ($Devices.Count -eq 0) {
            Write-Error "No authorized Android device found. Enable Developer Options + USB debugging, connect the phone, and accept the RSA prompt."
            exit 1
        }
        if ($Devices.Count -gt 1) {
            Write-Error "More than one Android device is attached. Set LINKDROP_ANDROID_SERIAL to one of:`n$($Devices -join "`n")"
            exit 1
        }
        $AdbTarget = @("-s", $Devices[0])
    }

    Write-Output "Installing $Apk"
    $InstallOutput = & $Adb @AdbTarget install -r $Apk 2>&1
    $InstallStatus = $LASTEXITCODE
    $InstallOutput | ForEach-Object { Write-Output $_ }

    if ($InstallStatus -ne 0) {
        $InstallText = $InstallOutput -join "`n"
        if ($InstallText -match "INSTALL_FAILED_UPDATE_INCOMPATIBLE") {
            Write-Error "The installed app was signed with a different certificate. If you are okay deleting local app data, uninstall it first:`n  $Adb $($AdbTarget -join " ") uninstall io.ahara.linkdrop"
        }
        exit $InstallStatus
    }
} finally {
    Pop-Location
}
