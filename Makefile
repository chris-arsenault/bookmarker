RUST_CLIPPY_FLAGS := -D warnings -W clippy::cognitive_complexity -W clippy::too_many_lines
RUST_MAX_FILE_LINES := 400
BACKEND_API_TESTS := api_capture api_foundation api_item_mutations api_items api_tags api_thumbnails
BACKEND_DB_SHARED_TESTS := library_pg_capture library_pg_items library_pg_tags linkdrop_capture_idempotency linkdrop_constraints linkdrop_inbox_status linkdrop_migration linkdrop_processing linkdrop_tags
BACKEND_DB_PROCESSING_TESTS := processing_pipeline

.PHONY: ci lint rust-lines-check fmt typecheck desktop-typecheck desktop-package test backend-fast-test frontend-test db-test android-structure-check android-build-check android-release-build android-assemble android-create-release-keystore android-sign-release android-install-debug android-install-release docs-check terraform-fmt-check build deploy

ci: lint fmt typecheck desktop-typecheck test android-structure-check android-build-check docs-check terraform-fmt-check

lint: rust-lines-check
	cd backend && cargo clippy --workspace --all-targets -- $(RUST_CLIPPY_FLAGS)
	cd frontend && pnpm exec eslint .

rust-lines-check:
	@violations=$$(find backend -path "*/target" -prune -o -name "*.rs" -type f -print | while IFS= read -r file; do lines=$$(wc -l < "$$file"); if [ "$$lines" -gt "$(RUST_MAX_FILE_LINES)" ]; then printf "%s %s\n" "$$lines" "$$file"; fi; done); \
	if [ -n "$$violations" ]; then printf "Rust files exceed %s lines:\n%s\n" "$(RUST_MAX_FILE_LINES)" "$$violations"; exit 1; fi

fmt:
	cd backend && cargo fmt -- --check
	cd frontend && pnpm exec prettier --check .

typecheck:
	cd frontend && pnpm exec tsc --noEmit

desktop-typecheck:
	cd frontend && pnpm run desktop:typecheck

desktop-package:
	cd frontend && pnpm run desktop:package

test: backend-fast-test frontend-test

backend-fast-test:
	cd backend && cargo test --workspace --lib --bins
	cd backend && cargo test -p api $(addprefix --test ,$(BACKEND_API_TESTS))

frontend-test:
	cd frontend && pnpm exec vitest run

db-test:
	cd backend && cargo test -p shared $(addprefix --test ,$(BACKEND_DB_SHARED_TESTS))
	cd backend && cargo test -p processing $(addprefix --test ,$(BACKEND_DB_PROCESSING_TESTS))

android-structure-check:
	scripts/check-android-share-target.sh

android-build-check:
	$(MAKE) android-assemble BUILD_VARIANT=Debug

android-release-build:
	$(MAKE) android-assemble BUILD_VARIANT=Release

android-assemble:
	@SDK_ROOT="$${ANDROID_HOME:-$${ANDROID_SDK_ROOT:-$${HOME}/android-sdk}}"; \
	if [ ! -d "$$SDK_ROOT/platforms/android-36" ]; then \
		echo "Android SDK platform android-36 not found. Set ANDROID_HOME or ANDROID_SDK_ROOT, or install it under $$HOME/android-sdk."; \
		exit 1; \
	fi; \
	ANDROID_HOME="$$SDK_ROOT" ANDROID_SDK_ROOT="$$SDK_ROOT" android/gradlew -p android --no-daemon :app:assemble$${BUILD_VARIANT:-Debug}

android-create-release-keystore:
	scripts/android-create-release-keystore.sh

android-sign-release:
	scripts/android-sign-release.sh

android-install-debug:
	scripts/android-install.sh debug

android-install-release:
	scripts/android-install.sh release

docs-check:
	test -f README.md
	test -f AGENTS.md
	test -f CLAUDE.md
	test -f docs/README.md
	test -f docs/architecture.md
	test -f docs/development.md
	test -f docs/backlog.md
	test -f docs/adr/README.md
	test -f LINKDROP-PLAN.md

terraform-fmt-check:
	terraform fmt -check -recursive infrastructure/terraform/

build:
	cd backend && cargo build --workspace
	cd frontend && pnpm run build
	cd frontend && pnpm run desktop:build
	$(MAKE) android-build-check

deploy:
	scripts/deploy.sh
