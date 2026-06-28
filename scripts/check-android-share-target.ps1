#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = (Resolve-Path (Join-Path $ScriptDir "..")).Path
$AndroidDir = Join-Path $RootDir "android/app/src/main"
$JavaDir = Join-Path $AndroidDir "java/io/ahara/linkdrop"
$Manifest = Join-Path $AndroidDir "AndroidManifest.xml"
$Shortcuts = Join-Path $AndroidDir "res/xml/shortcuts.xml"

function Assert-FileMatch {
    param(
        [string] $Path,
        [string] $Pattern
    )

    $Content = Get-Content -LiteralPath $Path -Raw
    if ($Content -notmatch $Pattern) {
        Write-Error "Expected $Path to match /$Pattern/."
        exit 1
    }
}

function Test-TreeMatch {
    param(
        [string] $Path,
        [string] $Pattern
    )

    $Files = Get-ChildItem -LiteralPath $Path -Recurse -File
    foreach ($File in $Files) {
        $Content = Get-Content -LiteralPath $File.FullName -Raw
        if ($Content -match $Pattern) {
            return $true
        }
    }
    return $false
}

function Assert-TreeMatch {
    param(
        [string] $Path,
        [string] $Pattern
    )

    if (-not (Test-TreeMatch -Path $Path -Pattern $Pattern)) {
        Write-Error "Expected files under $Path to match /$Pattern/."
        exit 1
    }
}

Assert-FileMatch -Path $Manifest -Pattern "android.intent.action.SEND"
Assert-FileMatch -Path $Manifest -Pattern "android.intent.action.SEND_MULTIPLE"
Assert-FileMatch -Path $Manifest -Pattern 'android:mimeType="text/plain"'
Assert-FileMatch -Path $Manifest -Pattern 'android:mimeType="image/\*"'
Assert-FileMatch -Path $Manifest -Pattern 'android:name="\.share\.ShareActivity"'
Assert-FileMatch -Path $Manifest -Pattern 'android:name="\.LinkdropApplication"'
Assert-FileMatch -Path $Manifest -Pattern "android.app.shortcuts"
Assert-FileMatch -Path $Manifest -Pattern "@xml/shortcuts"

Assert-FileMatch -Path $Shortcuts -Pattern '<share-target android:targetClass="io\.ahara\.linkdrop\.share\.ShareActivity">'
Assert-FileMatch -Path $Shortcuts -Pattern 'android:mimeType="text/plain"'
Assert-FileMatch -Path $Shortcuts -Pattern 'android:mimeType="image/\*"'
Assert-FileMatch -Path $Shortcuts -Pattern "io\.ahara\.linkdrop\.category\.TEXT_SHARE_TARGET"
Assert-FileMatch -Path $Shortcuts -Pattern "io\.ahara\.linkdrop\.category\.IMAGE_SHARE_TARGET"

Assert-TreeMatch -Path (Join-Path $JavaDir "auth") -Pattern "interface AuthRepository|class StoredTokenAuthRepository"
Assert-TreeMatch -Path (Join-Path $JavaDir "auth") -Pattern "class AuthTokenStore"
Assert-TreeMatch -Path (Join-Path $JavaDir "auth") -Pattern "class CognitoAuthClient"
Assert-FileMatch -Path (Join-Path $JavaDir "auth/CognitoAuthClient.kt") -Pattern "SOFTWARE_TOKEN_MFA|MFA_SETUP|REFRESH_TOKEN_AUTH"
Assert-FileMatch -Path (Join-Path $JavaDir "api/LinkdropApiClient.kt") -Pattern 'Authorization", "Bearer'

Assert-TreeMatch -Path (Join-Path $JavaDir "api") -Pattern "class LinkdropApiClient"
Assert-TreeMatch -Path (Join-Path $JavaDir "api") -Pattern "client_capture_id"
Assert-TreeMatch -Path (Join-Path $JavaDir "api") -Pattern "CaptureImageUploadAttempt"
Assert-TreeMatch -Path (Join-Path $JavaDir "api") -Pattern "ImageUploadTarget"
Assert-TreeMatch -Path (Join-Path $JavaDir "api") -Pattern "createImageUpload"
Assert-TreeMatch -Path (Join-Path $JavaDir "api") -Pattern "uploadImage"
Assert-TreeMatch -Path (Join-Path $JavaDir "api") -Pattern "completeImageUpload"
Assert-TreeMatch -Path (Join-Path $JavaDir "api") -Pattern '"/items"|"/tags"'

Assert-TreeMatch -Path (Join-Path $JavaDir "share") -Pattern "ShareIntentParser"
Assert-TreeMatch -Path (Join-Path $JavaDir "share") -Pattern "ACTION_SEND_MULTIPLE"
Assert-TreeMatch -Path (Join-Path $JavaDir "share") -Pattern "SharedCapture.Image"
Assert-TreeMatch -Path (Join-Path $JavaDir "share") -Pattern "openInputStream"
Assert-TreeMatch -Path (Join-Path $JavaDir "share") -Pattern "OpenableColumns"
Assert-TreeMatch -Path (Join-Path $JavaDir "share") -Pattern "ShareTagState"
Assert-TreeMatch -Path (Join-Path $JavaDir "share") -Pattern "TagChipRow"
Assert-TreeMatch -Path (Join-Path $JavaDir "share") -Pattern "listTags"
Assert-TreeMatch -Path (Join-Path $JavaDir "share") -Pattern "selectedTagValues"
Assert-FileMatch -Path (Join-Path $JavaDir "LinkdropApplication.kt") -Pattern "class LinkdropApplication"
Assert-FileMatch -Path (Join-Path $JavaDir "LinkdropApplication.kt") -Pattern "ShareShortcutPublisher\.publish"
Assert-FileMatch -Path (Join-Path $JavaDir "share/ShareShortcutPublisher.kt") -Pattern "object ShareShortcutPublisher"
Assert-FileMatch -Path (Join-Path $JavaDir "share/ShareShortcutPublisher.kt") -Pattern "ShortcutManager"
Assert-FileMatch -Path (Join-Path $JavaDir "share/ShareShortcutPublisher.kt") -Pattern "setDynamicShortcuts"
Assert-FileMatch -Path (Join-Path $JavaDir "share/ShareShortcutPublisher.kt") -Pattern "setLongLived"
Assert-FileMatch -Path (Join-Path $JavaDir "share/ShareShortcutPublisher.kt") -Pattern "reportShortcutUsed"
Assert-FileMatch -Path (Join-Path $JavaDir "share/ShareActivity.kt") -Pattern "Intent\.EXTRA_SHORTCUT_ID"
Assert-FileMatch -Path (Join-Path $JavaDir "share/ShareActivity.kt") -Pattern "ShareShortcutPublisher\.reportUsed"

if (Test-TreeMatch -Path $JavaDir -Pattern "generated tag|inferred tag|auto-generated|auto generated|suggested tag") {
    Write-Error "Android share flow must not generate or infer tags"
    exit 1
}
