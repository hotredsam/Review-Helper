# Release: sign, notarize, package (macOS)

Review Helper ships as a signed + notarized `.app`/`.dmg` so it launches on a
clean Mac without Gatekeeper warnings. The config is wired; the actual signing
needs **your Apple Developer credentials** (a "Developer ID Application"
certificate), which can't be committed.

## One-time
1. Enroll in the Apple Developer Program; create a **Developer ID Application**
   certificate and install it in your login keychain.
2. Create an app-specific password for your Apple ID (appleid.apple.com).

## Step 1 — code-sign (Tauri)
Set the signing identity, then build. Tauri **code-signs** the `.app` with the
hardened runtime (`src-tauri/entitlements.plist`). It does **NOT** notarize.

```sh
export APPLE_SIGNING_IDENTITY="Developer ID Application: <Your Name> (<TEAMID>)"
npm run tauri build
```

The signed (un-notarized) artifacts land in
`src-tauri/target/release/bundle/{macos,dmg}/`.

## Step 2 — notarize + staple (manual)
Submit the build to Apple, wait for approval, then staple the ticket so it
passes Gatekeeper offline:

```sh
APP="src-tauri/target/release/bundle/macos/Review Helper.app"
ditto -c -k --keepParent "$APP" /tmp/ReviewHelper.zip   # notarytool wants an archive
xcrun notarytool submit /tmp/ReviewHelper.zip \
  --apple-id "you@example.com" \
  --password "<app-specific-password>" \
  --team-id "<TEAMID>" \
  --wait
xcrun stapler staple "$APP"
```

## Verify (on a clean Mac)
```sh
spctl -a -vvv "Review Helper.app"      # should say: accepted, source=Notarized Developer ID
codesign --verify --deep --strict --verbose=2 "Review Helper.app"
```

> Note: the maintainer ran everything up to this point; the final sign +
> notarize step requires the Apple Developer ID certificate above and was left
> for the credential holder to run.
