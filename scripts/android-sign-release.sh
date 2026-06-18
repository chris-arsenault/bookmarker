#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SDK_ROOT="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-${HOME}/android-sdk}}"
BUILD_TOOLS="${LINKDROP_ANDROID_BUILD_TOOLS:-36.0.0}"
ZIPALIGN="${SDK_ROOT}/build-tools/${BUILD_TOOLS}/zipalign"
APKSIGNER="${SDK_ROOT}/build-tools/${BUILD_TOOLS}/apksigner"
KEYSTORE_PATH="${LINKDROP_ANDROID_KEYSTORE:-${HOME}/.android/linkdrop/linkdrop-release.jks}"
KEY_ALIAS="${LINKDROP_ANDROID_KEY_ALIAS:-linkdrop}"
RELEASE_DIR="${ROOT_DIR}/android/app/build/outputs/apk/release"

if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
    cat <<EOF
Usage: scripts/android-sign-release.sh

Builds the unsigned Linkdrop release APK, zipaligns it, signs it with:
  ${KEYSTORE_PATH}

Environment overrides:
  ANDROID_HOME / ANDROID_SDK_ROOT
  LINKDROP_ANDROID_BUILD_TOOLS
  LINKDROP_ANDROID_KEYSTORE
  LINKDROP_ANDROID_KEY_ALIAS
EOF
    exit 0
fi

require_file() {
    local path="$1"
    local hint="$2"
    if [ ! -f "${path}" ]; then
        echo "${path} not found. ${hint}" >&2
        exit 1
    fi
}

require_file "${ZIPALIGN}" "Install Android build-tools ${BUILD_TOOLS}."
require_file "${APKSIGNER}" "Install Android build-tools ${BUILD_TOOLS}."
require_file "${KEYSTORE_PATH}" "Run scripts/android-create-release-keystore.sh first."

cd "${ROOT_DIR}"
make android-release-build

mapfile -t unsigned_apks < <(find "${RELEASE_DIR}" -maxdepth 1 -type f -name 'linkdrop-release-unsigned-*.apk' | sort)
if [ "${#unsigned_apks[@]}" -ne 1 ]; then
    echo "Expected exactly one unsigned release APK in ${RELEASE_DIR}, found ${#unsigned_apks[@]}." >&2
    printf '  %s\n' "${unsigned_apks[@]}" >&2
    exit 1
fi

unsigned_apk="${unsigned_apks[0]}"
signed_apk="${unsigned_apk/-release-unsigned-/-release-signed-}"
aligned_apk="$(mktemp "${TMPDIR:-/tmp}/linkdrop-release-aligned.XXXXXX.apk")"
password=""

cleanup() {
    rm -f "${aligned_apk}"
    unset password
}
trap cleanup EXIT

if [ -t 0 ]; then
    read -r -s -p "Keystore password for ${KEYSTORE_PATH}: " password
    echo ""
else
    read -r password
fi

if [ -z "${password}" ]; then
    echo "Keystore password cannot be empty." >&2
    exit 1
fi

"${ZIPALIGN}" -p -f 4 "${unsigned_apk}" "${aligned_apk}"

printf '%s\n' "${password}" | "${APKSIGNER}" sign \
    --v4-signing-enabled false \
    --ks "${KEYSTORE_PATH}" \
    --ks-key-alias "${KEY_ALIAS}" \
    --ks-pass stdin \
    --out "${signed_apk}" \
    "${aligned_apk}"

"${APKSIGNER}" verify --print-certs "${signed_apk}"

echo ""
echo "Signed release APK:"
echo "  ${signed_apk}"
