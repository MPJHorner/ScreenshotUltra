---
title: "Install"
description: "Install Screenshot Ultra on macOS — one-line installer, manual DMG, or build from source."
slug: "install"
---

# Install

## One-liner

```sh
curl -sSL https://raw.githubusercontent.com/MPJHorner/ScreenshotUltra/main/scripts/install.sh | bash
```

What it does:

1. Detects your arch (arm64 / x86_64).
2. Reads the latest tag from the GitHub API.
3. Downloads the matching universal `.zip` from [Releases]({{repo}}/releases/latest).
4. Verifies the `.sha256` sidecar.
5. Removes any previous `/Applications/Screenshot Ultra.app` and unpacks the new one via `ditto -xk`.
6. Clears the `com.apple.quarantine` xattr so Gatekeeper doesn't gripe on first launch.
7. Opens the app once so macOS prompts you for **Screen Recording** permission.

## Manual: download from a GitHub release

1. Grab `ScreenshotUltra-v*-universal.zip` from [the latest release]({{repo}}/releases/latest).
2. Unzip into `/Applications/`.
3. Clear quarantine:

   ```sh
   xattr -dr com.apple.quarantine "/Applications/Screenshot Ultra.app"
   ```

4. Launch and grant **Screen Recording** in System Settings → Privacy & Security → Screen & System Audio Recording. Relaunch.

The binary is universal (arm64 + x86_64), unsigned for now. Signing + notarisation lands in M6.

## Build from source

Rust stable, macOS 13 or newer:

```sh
git clone https://github.com/MPJHorner/ScreenshotUltra.git
cd ScreenshotUltra
make app
open "dist/Screenshot Ultra.app"
```

`make app` runs:

- `cargo build --release`
- `scripts/render-icon.sh` (regenerates `icon/AppIcon.icns` from `icon/icon.svg` via Swift + `iconutil`)
- Assembles the `.app` bundle in `dist/`

## Permissions

- **Screen Recording** is required for every capture mode. macOS prompts on first launch; if you click "Don't Allow", re-enable it in System Settings → Privacy & Security → Screen & System Audio Recording.
- **Accessibility** is *not* required for the default hotkey scheme. It'll be needed for the planned modifier-only (chord) bindings — but those don't ship yet.

## Uninstall

```sh
# 1. Quit Screenshot Ultra from the menu bar (or kill the process)
pkill -f screenshot-ultra

# 2. Remove the app bundle
rm -rf "/Applications/Screenshot Ultra.app"

# 3. (Optional) wipe settings + logs
rm -rf "$HOME/Library/Application Support/ScreenshotUltra"
rm -rf "$HOME/Library/Logs/ScreenshotUltra"
```

Captures in `~/Pictures/ScreenshotUltra/` are left alone — they're yours.
