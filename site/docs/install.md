# Install

## One-liner

```sh
curl -sSL https://raw.githubusercontent.com/MPJHorner/ScreenshotUltra/main/scripts/install.sh | bash
```

What it does:

1. Detects your CPU (arm64 / x86_64).
2. Reads the latest tag from the GitHub API.
3. Downloads the matching universal `.zip` (or `.dmg` once signed releases ship).
4. Copies `Screenshot Ultra.app` into `/Applications`.
5. Removes the `com.apple.quarantine` xattr (so you don't get the
   "downloaded from internet" prompt every time).
6. Launches the app once so macOS prompts you for **Screen Recording**
   permission.

If no GitHub release exists yet (early development), the installer falls
back to building from source — that requires Rust (`rustup`).

## Manual: from a GitHub release

1. Download the latest `ScreenshotUltra-v*.zip` from
   [Releases](https://github.com/MPJHorner/ScreenshotUltra/releases).
2. Unzip and drag `Screenshot Ultra.app` into `/Applications`.
3. Remove quarantine: `xattr -dr com.apple.quarantine "/Applications/Screenshot Ultra.app"`
4. Launch and grant Screen Recording in System Settings → Privacy & Security
   → Screen & System Audio Recording. Relaunch the app.

## Build from source

```sh
git clone https://github.com/MPJHorner/ScreenshotUltra.git
cd ScreenshotUltra
make app
open "dist/Screenshot Ultra.app"
```

Requires Rust stable and macOS 13+ (Ventura). Universal binaries can be
built locally with `rustup target add aarch64-apple-darwin x86_64-apple-darwin`
followed by `cargo build --release --target <triple>` and `lipo -create`
(see [`.github/workflows/release.yml`](https://github.com/MPJHorner/ScreenshotUltra/blob/main/.github/workflows/release.yml)
for the canonical recipe).

## Permissions

- **Screen Recording** is required for any capture. macOS will prompt on
  first launch; if you click "Don't Allow" you can re-enable it later in
  System Settings → Privacy & Security → Screen & System Audio Recording.
- **Accessibility** is not required for the default hotkey scheme. It
  will be needed in a future milestone for modifier-only (chord) bindings.
