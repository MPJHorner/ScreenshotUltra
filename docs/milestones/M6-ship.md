# M6 — Ship v1.0

**Status:** 🔒 parked — needs a Developer ID certificate to test

The signing + notarisation pipeline can't be exercised meaningfully
without an Apple Developer Program account ($99/yr) and the resulting
Developer ID Application certificate. The build infrastructure (CI
release workflow → universal `.zip` + SHA-256, install.sh one-liner)
already ships every release; the signing pipeline is a future
add-on once a cert is available.

Until then the app:
- Ships as an unsigned universal `.zip`
- Asks users to clear `com.apple.quarantine` once
  (`install.sh` does this for them)
- Is auto-update-aware via the opt-in check (v0.9.0+) — users get a
  notification when a new release is available, but no automatic
  installation since Gatekeeper would reject an unsigned upgrade
  delivered behind their back

**Goal:** Cut a signed, notarised v1.0.0 release with the docs site live.

## Scope (from plan.md §13)

- [ ] Documentation site complete (mkdocs-material, parity with MailBox /
      Postbin Ultra). See plan.md §8 for the full IA.
- [ ] `scripts/codesign.sh` — Developer ID signing from `KEYCHAIN_PROFILE`.
- [ ] `scripts/notarize.sh` — `notarytool` submission + stapling.
- [ ] `scripts/build-dmg.sh` — `hdiutil` packaging with custom background.
- [ ] `.github/workflows/release.yml` — tag → DMG + tar.gz, attached to GH
      release. Signs/notarises only on the canonical repo (forks build
      unsigned).
- [ ] Homebrew cask at `MPJHorner/homebrew-ultra`.
- [ ] First-run onboarding window: Screen Recording permission, Accessibility
      permission (for chord hotkeys), default save folder, default capture
      action. Skippable in 5 seconds.
- [ ] Auto-update opt-in: default **off**, manual "Check for Updates" menu
      item; no telemetry, no phone-home (plan.md §11).
- [ ] CHANGELOG.md updated; tag `v1.0.0`.

## Success criteria (plan.md §16)

- [ ] Hotkey → clipboard image in < 100 ms on an M-series Mac, measured.
- [ ] Every advertised capture mode works on macOS 13 / 14 / 15 + latest.
- [ ] Zero network calls observable with Little Snitch in default config.
- [ ] DMG passes notarisation on a clean GH-hosted runner.
- [ ] Docs site renders, search works, every hotkey is documented.
- [ ] New user goes from `curl | bash` to first annotated screenshot in
      under 60 seconds without reading docs.
