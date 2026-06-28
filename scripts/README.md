# Scripts

Local automation for Linkdrop.

`make desktop-package` builds the frontend and Electron entrypoints, then writes
a runnable current-platform shell under `frontend/release/`. On Windows that
artifact is `Bookmarker.exe`; on Linux it is `Bookmarker`.

`scripts/deploy.sh` is the parameterless local deploy entry point. It builds the registered Rust Lambda release artifacts with `cargo lambda build --release`, builds the frontend, runs platform database migrations, applies Terraform with the shared Ahara state bucket defaults, and prints the frontend/API URLs from Terraform outputs.

`scripts/smoke.sh` checks the deployed API. Run it as `with-cred -- scripts/smoke.sh` so API tokens can be injected without writing secrets to disk. It always checks `/health`; when `LINKDROP_ACCESS_TOKEN` is present it also checks `/me`, `/items`, and `/tags`; when `LINKDROP_SMOKE_CAPTURE_URL` is present it performs an optional zero-tag capture smoke.

`scripts/android-create-release-keystore.sh` creates the local Linkdrop Android
release keystore at `$HOME/.android/linkdrop/linkdrop-release.jks` unless
`LINKDROP_ANDROID_KEYSTORE` overrides it. It refuses to overwrite an existing
keystore.

On Windows, use the PowerShell variant:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/android-create-release-keystore.ps1
```

`scripts/android-sign-release.sh` builds the unsigned release APK, aligns it,
signs it, verifies it, and writes a `linkdrop-release-signed-*.apk` artifact.
It prompts for the keystore password without requiring the password on the
command line.

On Windows:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/android-sign-release.ps1
```

`scripts/android-install.sh debug` builds and installs the debug APK.
`scripts/android-install.sh release` installs the signed release APK and runs
the signing script first when the signed artifact is missing. It requires
exactly one authorized device unless `LINKDROP_ANDROID_SERIAL` is set.

On Windows:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/android-install.ps1 debug
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/android-install.ps1 release
```
