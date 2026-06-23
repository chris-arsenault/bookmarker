# Frontend

Cognito-authenticated Vite React/TypeScript web vault for Bookmarker.

The interface browses saved URL, text-snippet, and image items, renders
feed/detail states, shows archive status, displays authenticated API-mediated
thumbnail snapshots when a stored `thumbnail_s3_key` exists, opens URL sources,
previews uploaded images, and copies canonical `url.copy_url` or snippet
`text.plain_text`. Filters cover platform, explicit tag, added date range,
`archive_status`, watched status, `inbox_status`, and free-text title/notes
search.

The Electron entrypoints in `electron/` load the built frontend, expose
clipboard IPC through `src/desktopBridge.ts`, and provide tray/HUD windows for
desktop clipboard capture without global keybinds.

Run `pnpm run desktop:package` to produce a runnable shell in `release/`. On
Windows the executable is `release/bookmarker-win32-x64/Bookmarker.exe`; on
Linux it is `release/bookmarker-linux-x64/Bookmarker`. The package script uses
the Electron runtime installed by `pnpm install` and fails before writing a
partial package if that runtime is missing.

The detail modal supports click-to-edit titles, blur-saved notes, chip-based
explicit tag replacement, watched/unwatched transitions, unsorted/organized
inbox transitions, source opening, copy actions, image download, and custom
delete confirmation. Tag management supports tag rename and tag merge over the
explicit tag corpus. Empty accounts start with no starter tags; chips appear
only after the user explicitly applies tags.
