# Android

Native Android client for Bookmarker quick-drop capture.

The app registers `ACTION_SEND` targets for `text/plain` and `image/*`, plus
`ACTION_SEND_MULTIPLE` for shared image batches. Shared text is parsed for the
first HTTP(S) URL; if no URL is present, non-empty shared text is captured as a
text snippet. Shared images are captured as image items, streamed from the
Android content URI to the API-issued upload target, then marked complete. The
user can drop any payload immediately with no required fields. The share screen
loads the authenticated `GET /tags` corpus
when available, renders most-used tags as optional chips, accepts a free-text
tag, and sends only selected or typed explicit tags.

The app also publishes Android Sharing Shortcuts for text/link and image drops.
These shortcuts back Direct Share targets in the system sharesheet once the app
process has started at least once, and successful shares report shortcut usage
back to Android for ranking.

The Android app signs in directly against the shared Ahara Cognito pool through
the Linkdrop public app client. It supports the platform software-token MFA
contract: existing users can answer `SOFTWARE_TOKEN_MFA`, and users who need
enrollment can complete `MFA_SETUP` from the one-time setup key before capture.
The stored refresh token is used to refresh access tokens before API calls.

URL capture calls `POST /items`; text capture calls `POST /items/text`; image
capture calls `POST /items/images/uploads`, uploads bytes to the returned URL,
then calls `POST /items/{id}/image-upload/complete`. All capture types send
selected explicit tags and a stable `client_capture_id` for the share attempt.
The API client requires a fresh bearer token from the local auth boundary before
making capture or tag corpus requests.

`make ci` runs both `android-structure-check` and a Gradle `:app:assembleDebug`
compile through the checked-in wrapper. Set `ANDROID_HOME` or `ANDROID_SDK_ROOT`
to an SDK that has platform `android-36`, or install the SDK under
`$HOME/android-sdk`.

APK outputs use product-specific names:

| Variant | Path                                                                           |
| ------- | ------------------------------------------------------------------------------ |
| Debug   | `android/app/build/outputs/apk/debug/linkdrop-debug-v0.1.0-1.apk`              |
| Release | `android/app/build/outputs/apk/release/linkdrop-release-unsigned-v0.1.0-1.apk` |

Run `make android-release-build` for the unsigned release variant. Release
signing is handled by the guarded signing script.

Use the guarded scripts for signing and device installs:

```bash
make android-create-release-keystore
make android-sign-release
make android-install-debug
make android-install-release
```

On Windows, the same Make targets dispatch to PowerShell scripts when
`OS=Windows_NT` is present. The scripts can also be run directly:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/android-create-release-keystore.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/android-sign-release.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/android-install.ps1 debug
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/android-install.ps1 release
```

The signing script writes
`android/app/build/outputs/apk/release/linkdrop-release-signed-v0.1.0-1.apk`.
It never stores the keystore password in the repo. Back up the keystore and
password; Android requires the same signing key for future upgrades.
