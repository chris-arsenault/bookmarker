#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ANDROID_DIR="${ROOT_DIR}/android/app/src/main"
JAVA_DIR="${ANDROID_DIR}/java/io/ahara/linkdrop"
MANIFEST="${ANDROID_DIR}/AndroidManifest.xml"
SHORTCUTS="${ANDROID_DIR}/res/xml/shortcuts.xml"

rg 'android.intent.action.SEND' "${MANIFEST}" >/dev/null
rg 'android.intent.action.SEND_MULTIPLE' "${MANIFEST}" >/dev/null
rg 'android:mimeType="text/plain"' "${MANIFEST}" >/dev/null
rg 'android:mimeType="image/\*"' "${MANIFEST}" >/dev/null
rg 'android:name=".share.ShareActivity"' "${MANIFEST}" >/dev/null
rg 'android:name=".LinkdropApplication"' "${MANIFEST}" >/dev/null
rg 'android.app.shortcuts' "${MANIFEST}" >/dev/null
rg '@xml/shortcuts' "${MANIFEST}" >/dev/null

rg '<share-target android:targetClass="io\.ahara\.linkdrop\.share\.ShareActivity">' "${SHORTCUTS}" >/dev/null
rg 'android:mimeType="text/plain"' "${SHORTCUTS}" >/dev/null
rg 'android:mimeType="image/\*"' "${SHORTCUTS}" >/dev/null
rg 'io\.ahara\.linkdrop\.category\.TEXT_SHARE_TARGET' "${SHORTCUTS}" >/dev/null
rg 'io\.ahara\.linkdrop\.category\.IMAGE_SHARE_TARGET' "${SHORTCUTS}" >/dev/null

rg 'interface AuthRepository|class StoredTokenAuthRepository' "${JAVA_DIR}/auth" >/dev/null
rg 'class AuthTokenStore' "${JAVA_DIR}/auth" >/dev/null
rg 'class CognitoAuthClient' "${JAVA_DIR}/auth" >/dev/null
rg 'SOFTWARE_TOKEN_MFA|MFA_SETUP|REFRESH_TOKEN_AUTH' "${JAVA_DIR}/auth/CognitoAuthClient.kt" >/dev/null
rg 'Authorization", "Bearer' "${JAVA_DIR}/api/LinkdropApiClient.kt" >/dev/null

rg 'class LinkdropApiClient' "${JAVA_DIR}/api" >/dev/null
rg 'client_capture_id' "${JAVA_DIR}/api" >/dev/null
rg 'CaptureImageUploadAttempt' "${JAVA_DIR}/api" >/dev/null
rg 'ImageUploadTarget' "${JAVA_DIR}/api" >/dev/null
rg 'createImageUpload' "${JAVA_DIR}/api" >/dev/null
rg 'uploadImage' "${JAVA_DIR}/api" >/dev/null
rg 'completeImageUpload' "${JAVA_DIR}/api" >/dev/null
rg '"/items"|"/tags"' "${JAVA_DIR}/api" >/dev/null

rg 'ShareIntentParser' "${JAVA_DIR}/share" >/dev/null
rg 'ACTION_SEND_MULTIPLE' "${JAVA_DIR}/share" >/dev/null
rg 'SharedCapture.Image' "${JAVA_DIR}/share" >/dev/null
rg 'openInputStream' "${JAVA_DIR}/share" >/dev/null
rg 'OpenableColumns' "${JAVA_DIR}/share" >/dev/null
rg 'ShareTagState' "${JAVA_DIR}/share" >/dev/null
rg 'TagChipRow' "${JAVA_DIR}/share" >/dev/null
rg 'listTags' "${JAVA_DIR}/share" >/dev/null
rg 'selectedTagValues' "${JAVA_DIR}/share" >/dev/null
rg 'class LinkdropApplication' "${JAVA_DIR}/LinkdropApplication.kt" >/dev/null
rg 'ShareShortcutPublisher\.publish' "${JAVA_DIR}/LinkdropApplication.kt" >/dev/null
rg 'object ShareShortcutPublisher' "${JAVA_DIR}/share/ShareShortcutPublisher.kt" >/dev/null
rg 'ShortcutManager' "${JAVA_DIR}/share/ShareShortcutPublisher.kt" >/dev/null
rg 'setDynamicShortcuts' "${JAVA_DIR}/share/ShareShortcutPublisher.kt" >/dev/null
rg 'setLongLived' "${JAVA_DIR}/share/ShareShortcutPublisher.kt" >/dev/null
rg 'reportShortcutUsed' "${JAVA_DIR}/share/ShareShortcutPublisher.kt" >/dev/null
rg 'Intent\.EXTRA_SHORTCUT_ID' "${JAVA_DIR}/share/ShareActivity.kt" >/dev/null
rg 'ShareShortcutPublisher\.reportUsed' "${JAVA_DIR}/share/ShareActivity.kt" >/dev/null

if rg 'generated tag|inferred tag|auto-generated|auto generated|suggested tag' "${JAVA_DIR}" >/dev/null; then
    echo "Android share flow must not generate or infer tags" >&2
    exit 1
fi
