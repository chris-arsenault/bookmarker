# Changelog

All notable user-visible changes are recorded here.

## v0.1.0 - 2026-06-23

### Capture

- Added authenticated quick-drop capture for URLs, text snippets, and images with optional explicit tags.
- Added canonical URL storage for common tracking-parameter stripping, short-share normalization, and best-effort short-link resolution.
- Added per-user deduplication for normalized URLs, text content hashes, and image upload retries through stable client capture IDs.
- Added best-effort asynchronous URL metadata enrichment and Linkdrop-owned thumbnail snapshots.

### Library

- Added the authenticated web vault with list/detail browsing, filters, search, notes, tags, watched state, inbox state, copy actions, and source opening.
- Added user-entered titles for URL, text, and image items while keeping fetched provider titles separate.
- Added compact inline detail editing with title-on-click, blur-saved notes, chip-based tag selection, and status icon popovers.
- Added authenticated image reads so uploaded phone images can be viewed and downloaded from desktop.
- Added tag rename and tag merge workflows that preserve item associations and usage counts.

### Clients

- Added a native Android share target for URL, text, and image payloads with Cognito authentication and software-token MFA support.
- Added an Electron desktop shell for explicit clipboard capture, tray access, authenticated vault browsing, and a compact HUD.
- Added Windows and Linux desktop packaging through the local Electron runtime.

### Operations

- Added Ahara deployment wiring for the API Lambda, processing Lambda, website, Cognito app client, auth-trigger registration, private snapshot bucket, runtime config, CloudWatch alarms, local deploy outputs, and smoke checks.
- Added guarded Android release keystore, signing, and device-install scripts.

### Bug fixes

- Fixed title preservation so metadata enrichment surfaces fetched titles separately from user-entered titles.
- Fixed link item deletion so delete state does not remain stuck across selected items.
- Fixed desktop sign-in persistence and callback handling for the packaged Electron shell.
