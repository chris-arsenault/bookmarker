#!/usr/bin/env bash
set -euo pipefail

KEYSTORE_PATH="${LINKDROP_ANDROID_KEYSTORE:-${HOME}/.android/linkdrop/linkdrop-release.jks}"
KEY_ALIAS="${LINKDROP_ANDROID_KEY_ALIAS:-linkdrop}"
DNAME="${LINKDROP_ANDROID_DNAME:-CN=Linkdrop, OU=Linkdrop, O=Ahara, C=US}"

if ! command -v keytool >/dev/null 2>&1; then
    echo "keytool not found. Install a JDK before creating the release keystore." >&2
    exit 1
fi

if [ -e "${KEYSTORE_PATH}" ]; then
    echo "Release keystore already exists:"
    echo "  ${KEYSTORE_PATH}"
    echo "Refusing to overwrite it."
    exit 0
fi

umask 077
mkdir -p "$(dirname "${KEYSTORE_PATH}")"

echo "Creating Linkdrop Android release keystore:"
echo "  ${KEYSTORE_PATH}"
echo ""
echo "Use a strong password and store it somewhere durable."
echo "When keytool asks for the key password, press RETURN to reuse the keystore password."
echo ""

keytool -genkeypair -v \
    -keystore "${KEYSTORE_PATH}" \
    -alias "${KEY_ALIAS}" \
    -keyalg RSA \
    -keysize 4096 \
    -validity 10000 \
    -storetype PKCS12 \
    -dname "${DNAME}"

chmod 600 "${KEYSTORE_PATH}"

echo ""
echo "Created ${KEYSTORE_PATH}"
echo "Keep this file and its password. Losing it prevents future upgrades of the same app id."
