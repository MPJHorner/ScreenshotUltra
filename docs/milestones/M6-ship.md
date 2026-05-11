# M6 — Ship v1.0

**Status:** ⏳ planned

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
