#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SDK_ROOT="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-${HOME}/android-sdk}}"
ADB="${SDK_ROOT}/platform-tools/adb"
MODE="${1:-debug}"

usage() {
    cat <<EOF
Usage: scripts/android-install.sh [debug|release]

Installs the Linkdrop APK onto exactly one connected Android device.

Environment overrides:
  ANDROID_HOME / ANDROID_SDK_ROOT
  LINKDROP_ANDROID_SERIAL    Device serial when more than one device is attached
EOF
}

if [ "${MODE}" = "-h" ] || [ "${MODE}" = "--help" ]; then
    usage
    exit 0
fi

if [ "${MODE}" != "debug" ] && [ "${MODE}" != "release" ]; then
    usage >&2
    exit 1
fi

if [ ! -x "${ADB}" ]; then
    echo "adb not found at ${ADB}. Set ANDROID_HOME or ANDROID_SDK_ROOT." >&2
    exit 1
fi

cd "${ROOT_DIR}"

if [ "${MODE}" = "debug" ]; then
    make android-build-check
    apk="android/app/build/outputs/apk/debug/linkdrop-debug-v0.1.0-1.apk"
else
    release_dir="android/app/build/outputs/apk/release"
    mapfile -t signed_apks < <(find "${release_dir}" -maxdepth 1 -type f -name 'linkdrop-release-signed-*.apk' | sort 2>/dev/null || true)
    if [ "${#signed_apks[@]}" -eq 0 ]; then
        echo "No signed release APK found. Running scripts/android-sign-release.sh first."
        scripts/android-sign-release.sh
        mapfile -t signed_apks < <(find "${release_dir}" -maxdepth 1 -type f -name 'linkdrop-release-signed-*.apk' | sort)
    fi
    if [ "${#signed_apks[@]}" -ne 1 ]; then
        echo "Expected exactly one signed release APK in ${release_dir}, found ${#signed_apks[@]}." >&2
        printf '  %s\n' "${signed_apks[@]}" >&2
        exit 1
    fi
    apk="${signed_apks[0]}"
fi

if [[ "${apk}" == *unsigned* ]]; then
    echo "Refusing to install an unsigned release APK: ${apk}" >&2
    exit 1
fi

if [ ! -f "${apk}" ]; then
    echo "APK not found: ${apk}" >&2
    exit 1
fi

if [ -n "${LINKDROP_ANDROID_SERIAL:-}" ]; then
    adb_target=(-s "${LINKDROP_ANDROID_SERIAL}")
else
    mapfile -t devices < <("${ADB}" devices | awk 'NR > 1 && $2 == "device" { print $1 }')
    if [ "${#devices[@]}" -eq 0 ]; then
        echo "No authorized Android device found." >&2
        echo "Enable Developer Options + USB debugging, connect the phone, and accept the RSA prompt." >&2
        exit 1
    fi
    if [ "${#devices[@]}" -gt 1 ]; then
        echo "More than one Android device is attached. Set LINKDROP_ANDROID_SERIAL to one of:" >&2
        printf '  %s\n' "${devices[@]}" >&2
        exit 1
    fi
    adb_target=(-s "${devices[0]}")
fi

echo "Installing ${apk}"
set +e
install_output="$("${ADB}" "${adb_target[@]}" install -r "${apk}" 2>&1)"
install_status=$?
set -e

printf '%s\n' "${install_output}"

if [ "${install_status}" -ne 0 ]; then
    if grep -q 'INSTALL_FAILED_UPDATE_INCOMPATIBLE' <<<"${install_output}"; then
        echo "" >&2
        echo "The installed app was signed with a different certificate." >&2
        echo "If you are okay deleting local app data, uninstall it first:" >&2
        echo "  ${ADB} ${adb_target[*]} uninstall io.ahara.linkdrop" >&2
    fi
    exit "${install_status}"
fi
