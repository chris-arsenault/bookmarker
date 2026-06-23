# Bookmarker

Bookmarker is an Ahara-backed personal capture vault for links, text snippets, and images across Android, web, and desktop.

## Quickstart

```bash
cd frontend
pnpm install --frozen-lockfile
cd ..
make ci
```

`make ci` includes the Android debug build. Set `ANDROID_HOME` or
`ANDROID_SDK_ROOT` to an SDK with platform `android-36`, or install that SDK
under `$HOME/android-sdk`.

Run focused database tests for migration, PostgreSQL repository, or processing
queue changes:

```bash
make db-test
```

Package the desktop shell:

```bash
make desktop-package
```

## URLs

| Surface | URL                             |
| ------- | ------------------------------- |
| App     | `https://linkdrop.ahara.io`     |
| API     | `https://api.linkdrop.ahara.io` |

## Documentation

| Topic                  | Link                                         |
| ---------------------- | -------------------------------------------- |
| Architecture           | [docs/architecture.md](docs/architecture.md) |
| Development            | [docs/development.md](docs/development.md)   |
| Architecture decisions | [docs/adr/README.md](docs/adr/README.md)     |
| Backlog                | [docs/backlog.md](docs/backlog.md)           |
| Changelog              | [CHANGELOG.md](CHANGELOG.md)                 |
| Implementation plan    | [LINKDROP-PLAN.md](LINKDROP-PLAN.md)         |
| Agent guide            | [AGENTS.md](AGENTS.md)                       |

## License

MIT. See [LICENSE](LICENSE).
