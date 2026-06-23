# Agent Guide

Bookmarker is an Ahara-integrated personal capture vault with a Rust API, React web UI, Electron desktop clipboard shell, and native Android share target.

## Read first

| Topic                  | Link                                                                             |
| ---------------------- | -------------------------------------------------------------------------------- |
| Workspace overview     | [README.md](README.md)                                                           |
| Implementation plan    | [LINKDROP-PLAN.md](LINKDROP-PLAN.md)                                             |
| Documentation index    | [docs/README.md](docs/README.md)                                                 |
| Architecture           | [docs/architecture.md](docs/architecture.md)                                     |
| Architecture decisions | [docs/adr/README.md](docs/adr/README.md)                                         |
| Backlog                | [docs/backlog.md](docs/backlog.md)                                               |
| Changelog              | [CHANGELOG.md](CHANGELOG.md)                                                     |
| Platform integration   | [../ahara/INTEGRATION.md](../ahara/INTEGRATION.md)                               |
| Ahara standards        | [../ahara-standards/standards/README.md](../ahara-standards/standards/README.md) |

## Critical rules

- Follow the Ahara platform contract: shared ALB, shared VPC, shared PostgreSQL/RDS, shared Cognito, shared Terraform state, and `ahara-tf-patterns` modules.
- Keep capture tax at zero mandatory fields: a shared link or text snippet is saved immediately, and tagging remains optional.
- Store user-applied tags only. The tag corpus is built from explicit user choices and never from inferred or generated tags.
- Normalize source URLs before persistence, deduplicate URL items by normalized URL, deduplicate text snippets by content hash, and keep the canonical source link on URL items.
- Treat enrichment and thumbnail archival as best-effort processing. Failed enrichment still leaves a saved item with visible status.
- Store thumbnail snapshots as Linkdrop-owned copies, not hotlinks to source platforms.
- Keep Android as a native client under `android/`; it consumes the same authenticated API as the web frontend and can capture URL, text, or image share payloads.
- Start local development servers only when the user explicitly asks.
- Run `make ci` before handoff after changing files.
- Run normal Git remote operations directly. `git fetch`, `git pull`, and `git push` use the configured SSH remote/agent. Use the secret broker only for commands that need injected runtime secrets such as AWS, Cognito, database, live API, or smoke-test credentials.

## Code map

| Path                        | Purpose                                                                                                                    |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `backend/`                  | Rust Lambda workspace for API, async processing, and shared domain code                                                    |
| `frontend/`                 | Vite React/TypeScript SPA plus Electron desktop shell entrypoints for the vault, clipboard capture, HUD, and management UI |
| `android/`                  | Native Android share target and quick-drop client                                                                          |
| `db/migrations/`            | PostgreSQL migrations for the Bookmarker/Linkdrop project database                                                         |
| `infrastructure/terraform/` | Project Terraform root using Ahara platform modules                                                                        |
| `docs/`                     | Architecture, development notes, ADRs, and backlog                                                                         |
| `scripts/`                  | Local automation added by implementation milestones                                                                        |

## Commands

| Command        | Purpose                                                                                          |
| -------------- | ------------------------------------------------------------------------------------------------ |
| `make ci`      | Run the canonical local verification target for the current repo state                           |
| `make db-test` | Run Docker-backed PostgreSQL migration, repository, and processing integration tests             |
| `make build`   | Build the registered Rust workspace, frontend shell, Electron entrypoints, and Android debug APK |
| `make deploy`  | Run the parameterless local deploy script                                                        |

`make ci` enforces Rust Clippy warnings, Rust cognitive complexity `10`, Rust function length `75`, Rust source files under `400` lines, TypeScript/React cognitive complexity `10`, TypeScript files under `400` lines, and TypeScript functions under `75` lines. It intentionally excludes the Docker-backed PostgreSQL integration suite; run `make db-test` for migration, PostgreSQL repository, or processing queue changes.
