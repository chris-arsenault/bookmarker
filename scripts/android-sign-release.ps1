#Requires -Version 5.1
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

function Get-AndroidToolPath {
    param(
        [string] $Directory,
        [string] $BaseName
    )

    $Candidates = @("$BaseName.exe", "$BaseName.bat", $BaseName)
    foreach ($Candidate in $Candidates) {
        $Path = Join-Path $Directory $Candidate
        if (Test-Path -LiteralPath $Path -PathType Leaf) {
            return $Path
        }
    }
    return Join-Path $Directory $BaseName
}

function Require-File {
    param(
        [string] $Path,
        [string] $Hint
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Write-Error "$Path not found. $Hint"
        exit 1
    }
}

function ConvertTo-PlainText {
    param([Security.SecureString] $SecureValue)

    $Pointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($SecureValue)
    try {
        return [Runtime.InteropServices.Marshal]::PtrToStringBSTR($Pointer)
    } finally {
        [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($Pointer)
    }
}

function Read-KeystorePassword {
    param([string] $KeystorePath)

    if ([Console]::IsInputRedirected) {
        return [Console]::In.ReadLine()
    }
    $SecurePassword = Read-Host -Prompt "Keystore password for $KeystorePath" -AsSecureString
    return ConvertTo-PlainText -SecureValue $SecurePassword
}

$SdkRoot = Get-AndroidSdkRoot
$BuildTools = if ([string]::IsNullOrWhiteSpace($env:LINKDROP_ANDROID_BUILD_TOOLS)) {
    "36.0.0"
} else {
    $env:LINKDROP_ANDROID_BUILD_TOOLS
}
$BuildToolsDir = Join-Path (Join-Path $SdkRoot "build-tools") $BuildTools
$Zipalign = Get-AndroidToolPath -Directory $BuildToolsDir -BaseName "zipalign"
$ApkSigner = Get-AndroidToolPath -Directory $BuildToolsDir -BaseName "apksigner"
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
$ReleaseDir = Join-Path $RootDir "android/app/build/outputs/apk/release"

Require-File -Path $Zipalign -Hint "Install Android build-tools $BuildTools."
Require-File -Path $ApkSigner -Hint "Install Android build-tools $BuildTools."
Require-File -Path $KeystorePath -Hint "Run scripts/android-create-release-keystore.ps1 first."

Push-Location $RootDir
try {
    & (Join-Path $ScriptDir "android-assemble.ps1") -BuildVariant Release
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    $UnsignedApks = @(Get-ChildItem -LiteralPath $ReleaseDir -Filter "linkdrop-release-unsigned-*.apk" -File | Sort-Object FullName)
    if ($UnsignedApks.Count -ne 1) {
        Write-Error "Expected exactly one unsigned release APK in $ReleaseDir, found $($UnsignedApks.Count).`n$($UnsignedApks.FullName -join "`n")"
        exit 1
    }

    $UnsignedApk = $UnsignedApks[0].FullName
    $SignedApk = $UnsignedApk -replace "-release-unsigned-", "-release-signed-"
    $AlignedApk = Join-Path ([IO.Path]::GetTempPath()) "linkdrop-release-aligned-$([Guid]::NewGuid()).apk"
    $Password = ""

    try {
        $Password = Read-KeystorePassword -KeystorePath $KeystorePath
        if ([string]::IsNullOrEmpty($Password)) {
            Write-Error "Keystore password cannot be empty."
            exit 1
        }

        & $Zipalign -p -f 4 $UnsignedApk $AlignedApk
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }

        $Password | & $ApkSigner sign `
            --v4-signing-enabled false `
            --ks $KeystorePath `
            --ks-key-alias $KeyAlias `
            --ks-pass stdin `
            --out $SignedApk `
            $AlignedApk
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }

        & $ApkSigner verify --print-certs $SignedApk
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }

        Write-Output ""
        Write-Output "Signed release APK:"
        Write-Output "  $SignedApk"
    } finally {
        if (Test-Path -LiteralPath $AlignedApk) {
            Remove-Item -LiteralPath $AlignedApk -Force
        }
        $Password = $null
    }
} finally {
    Pop-Location
}
