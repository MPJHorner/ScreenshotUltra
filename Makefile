# Screenshot Ultra — Make targets (mirrors MailBox / Postbin Ultra)

CARGO        ?= cargo
APP_NAME     := Screenshot Ultra
BIN_NAME     := screenshot-ultra
BUNDLE_ID    := com.mpjhorner.ScreenshotUltra
TARGET_DIR   := target
RELEASE_DIR  := $(TARGET_DIR)/release
DIST_DIR     := dist
APP_DIR      := $(DIST_DIR)/$(APP_NAME).app

.PHONY: run check fmt clippy test build release app clean help

help:
	@echo "make run      — cargo run (dev)"
	@echo "make check    — fmt + clippy + test"
	@echo "make build    — cargo build --release"
	@echo "make app      — build .app bundle in ./dist"
	@echo "make clean    — remove ./target and ./dist"

run:
	$(CARGO) run

fmt:
	$(CARGO) fmt --all

clippy:
	$(CARGO) clippy --all-targets -- -D warnings

test:
	$(CARGO) test --all

check: fmt clippy test

build:
	$(CARGO) build --release

app: build icon/AppIcon.icns
	@rm -rf "$(APP_DIR)"
	@mkdir -p "$(APP_DIR)/Contents/MacOS" "$(APP_DIR)/Contents/Resources"
	@cp "$(RELEASE_DIR)/$(BIN_NAME)" "$(APP_DIR)/Contents/MacOS/$(BIN_NAME)"
	@cp mac/Info.plist "$(APP_DIR)/Contents/Info.plist"
	@cp icon/AppIcon.icns "$(APP_DIR)/Contents/Resources/AppIcon.icns"
	@echo "APPL????" > "$(APP_DIR)/Contents/PkgInfo"
	@echo "Built $(APP_DIR)"

# Render icon/icon.svg into a full AppIcon.icns via Swift's NSImage + iconutil.
# Regenerate by deleting AppIcon.icns and re-running `make app`.
icon/AppIcon.icns: icon/icon.svg scripts/render-icon.sh
	@bash scripts/render-icon.sh

clean:
	$(CARGO) clean
	rm -rf "$(DIST_DIR)"
