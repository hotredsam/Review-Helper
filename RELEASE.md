# Release: sign, notarize, package (macOS)

Review Helper ships as a signed + notarized `.app`/`.dmg` so it launches on a
clean Mac without Gatekeeper warnings. The config is wired; the actual signing
needs **your Apple Developer credentials** (a "Developer ID Application"
certificate), which can't be committed.

## One-time
1. Enroll in the Apple Developer Program; create a **Developer ID Application**
   certificate and install it in your login keychain.
2. Create an app-specific password for your Apple ID (appleid.apple.com).

## Build + notarize
Set these env vars, then run the bundler — Tauri signs with the hardened runtime
(`src-tauri/entitlements.plist`) and submits for notarization:

```sh
export APPLE_SIGNING_IDENTITY="Developer ID Application: <Your Name> (<TEAMID>)"
export APPLE_ID="you@example.com"
export APPLE_PASSWORD="<app-specific-password>"
export APPLE_TEAM_ID="<TEAMID>"

npm run tauri build
```

The signed, notarized artifacts land in
`src-tauri/target/release/bundle/{macos,dmg}/`.

## Verify (on a clean Mac)
```sh
spctl -a -vvv "Review Helper.app"      # should say: accepted, source=Notarized Developer ID
codesign --verify --deep --strict --verbose=2 "Review Helper.app"
```

> Note: the maintainer ran everything up to this point; the final sign +
> notarize step requires the Apple Developer ID certificate above and was left
> for the credential holder to run.
