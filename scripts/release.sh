#!/bin/bash
#
# Builds, signs, notarizes and packages switchboard's macOS build, then
# tags the version and publishes it to a GitHub release.
#
# One-time setup:
#   1. Create a "Developer ID Application" certificate (NOT "Apple
#      Development" — a different type, and only the Account Holder can make
#      one): Xcode → Settings → Accounts → select your team → Manage
#      Certificates → + → Developer ID Application.
#   2. Create an app-specific password at https://appleid.apple.com → Sign-In
#      and Security → App-Specific Passwords.
#   3. Store notarization credentials once:
#        xcrun notarytool store-credentials switchboard \
#          --apple-id "<your-apple-id>" \
#          --team-id "<your-team-id>" \
#          --password "<app-specific-password>"
#
# Then: TEAM_ID=<your-team-id> ./scripts/release.sh
#
set -euo pipefail
cd "$(dirname "$0")/.."

# Taken from the environment so no maintainer's identity is baked into a
# public repository. TEAM_ID is the parenthesised code in the certificate
# name that `security find-identity -v -p codesigning` prints.
TEAM_ID="${TEAM_ID:-}"
NOTARY_PROFILE="${NOTARY_PROFILE:-switchboard}"
BUNDLE_ID="dev.udhayadithya.switchboard"
BUILD_DIR="build/release"
APP="$BUILD_DIR/switchboard.app"

say() { printf '\n\033[1m==> %s\033[0m\n' "$1"; }
fail() { printf '\n\033[31merror: %s\033[0m\n' "$1" >&2; exit 1; }

# --- Preflight ---------------------------------------------------------
say "Preflight"

if [ -z "$TEAM_ID" ]; then
  DETECTED=$(security find-identity -v -p codesigning 2>/dev/null \
    | sed -n 's/.*Developer ID Application: .*(\([A-Z0-9]*\)).*/\1/p' | head -1)
  if [ -n "$DETECTED" ]; then
    TEAM_ID="$DETECTED"
    echo "team: $TEAM_ID (detected from the signing certificate)"
  else
    fail "TEAM_ID is not set and no Developer ID certificate was found.

Set it explicitly:  TEAM_ID=XXXXXXXXXX ./scripts/release.sh"
  fi
fi

if ! security find-identity -v -p codesigning | grep -q "Developer ID Application"; then
  fail "No 'Developer ID Application' certificate found.

Create one: Xcode → Settings → Accounts → your team → Manage Certificates
        → + → Developer ID Application

This requires the Account Holder role on the paid developer account."
fi

# Apple's secure timestamp service, checked before anything is built.
# codesign contacts it for every signature, and without it the build fails
# several minutes in with "A timestamp was expected but was not found" — a
# message that names a symptom and not a cause. Probed by actually signing a
# throwaway file, since nothing simpler is truthful: RFC 3161 timestamping
# runs over plain HTTP on port 80, so a TCP check on 443 reports failure on
# networks where signing works fine.
TIMESTAMP_PROBE="$BUILD_DIR/.timestamp-probe"
mkdir -p "$BUILD_DIR"
cp /usr/bin/true "$TIMESTAMP_PROBE"

codesign --force --sign "Developer ID Application" -o runtime \
  --timestamp "$TIMESTAMP_PROBE" >/dev/null 2>&1 || true

PROBE_INFO=$(codesign -dvv "$TIMESTAMP_PROBE" 2>&1 || true)
rm -f "$TIMESTAMP_PROBE"

case "$PROBE_INFO" in
  *"Timestamp="*)
    echo "secure timestamp: available"
    ;;
  *)
    fail "codesign cannot obtain a secure timestamp from Apple.

Notarization requires one. The service is reached over HTTP on port 80 (not
443) at timestamp.apple.com — try another network, such as a phone hotspot,
to confirm this is a network problem rather than a configuration one."
    ;;
esac

# Distinguishes a missing profile from an unreachable Apple, because
# `notarytool history` is a network call and its failure otherwise reads as
# "your credentials are missing" — sending you to re-enter an app-specific
# password that was never the problem.
NOTARY_CHECK=$(xcrun notarytool history --keychain-profile "$NOTARY_PROFILE" 2>&1 || true)
case "$NOTARY_CHECK" in
  *NSURLErrorDomain*|*"timed out"*|*"could not connect"*|*"network connection"*)
    fail "Cannot reach Apple's notary service.

The '$NOTARY_PROFILE' profile may well be fine — this request did not get far
enough to find out:

$(printf '%s' "$NOTARY_CHECK" | head -2)"
    ;;
  *"Successfully received submission history"*|*"createdDate"*|*"No submissions"*|*"id:"*)
    : # reachable and authenticated
    ;;
  *)
    fail "Notarization profile '$NOTARY_PROFILE' could not be used. Run:

  xcrun notarytool store-credentials $NOTARY_PROFILE \\
    --apple-id \"<your-apple-id>\" --team-id $TEAM_ID --password \"<app-specific-password>\"

notarytool said:
$(printf '%s' "$NOTARY_CHECK" | head -3)"
    ;;
esac

command -v gh >/dev/null || fail "gh not installed (brew install gh)"

VERSION=$(awk -F'"' '/^version/{print $2; exit}' Cargo.toml)
[ -n "$VERSION" ] || fail "could not read version from Cargo.toml"
TAG="v$VERSION"
echo "version: $VERSION"

if git rev-parse "$TAG" >/dev/null 2>&1; then
  fail "Tag $TAG already exists — bump the version in Cargo.toml first."
fi

# --- Build ---------------------------------------------------------------
say "Building"
cargo build --release

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/switchboard "$APP/Contents/MacOS/switchboard"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>switchboard</string>
  <key>CFBundleDisplayName</key><string>switchboard</string>
  <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleExecutable</key><string>switchboard</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# --- Sign ------------------------------------------------------------------
say "Signing"
codesign --force --deep --options runtime --timestamp \
  --sign "Developer ID Application" "$APP"

say "Verifying signature"
codesign --verify --deep --strict --verbose=2 "$APP"

# Captured once into a variable rather than piped into grep: `grep -q` exits
# at its first match, which can kill codesign mid-write with SIGPIPE, and
# under `pipefail` that surfaces as a pipeline failure even though the match
# succeeded.
SIGNATURE_INFO=$(codesign -dv --verbose=4 "$APP" 2>&1 || true)
printf '%s\n' "$SIGNATURE_INFO" | grep -E "Authority|flags|TeamIdentifier" || true

# Hardened runtime is mandatory for notarization.
case "$SIGNATURE_INFO" in
  *"flags="*runtime*) echo "hardened runtime: enabled" ;;
  *) fail "Hardened runtime is not enabled; notarization will be rejected" ;;
esac

# An Apple Development signature builds and runs locally but can never be
# notarized, and the failure otherwise appears much later as an opaque
# rejection.
case "$SIGNATURE_INFO" in
  *"Developer ID Application"*) echo "signing identity: Developer ID" ;;
  *) fail "Not signed with a Developer ID Application certificate.

Signed with:
$(printf '%s\n' "$SIGNATURE_INFO" | grep Authority || echo '  (unknown)')" ;;
esac

# --- Package -----------------------------------------------------------
say "Building DMG ($VERSION)"
STAGING="$BUILD_DIR/dmg"
rm -rf "$STAGING"; mkdir -p "$STAGING"
cp -R "$APP" "$STAGING/"
ln -s /Applications "$STAGING/Applications"

DMG="$BUILD_DIR/switchboard-$VERSION-macos.dmg"
rm -f "$DMG"
hdiutil create -volname switchboard -srcfolder "$STAGING" -ov -format UDZO "$DMG" >/dev/null
codesign --sign "Developer ID Application" --timestamp "$DMG"

# --- Notarize ------------------------------------------------------------
say "Notarizing (this can take several minutes)"
xcrun notarytool submit "$DMG" --keychain-profile "$NOTARY_PROFILE" --wait

say "Stapling"
xcrun stapler staple "$DMG"
xcrun stapler validate "$DMG"

say "Verifying Gatekeeper acceptance"
spctl --assess --type open --context context:primary-signature -v "$DMG" 2>&1 || true

# --- Tag and publish -------------------------------------------------------
say "Tagging $TAG"
git tag -a "$TAG" -m "$TAG"
git push origin "$TAG"

say "Publishing to GitHub"
if gh release view "$TAG" >/dev/null 2>&1; then
  gh release upload "$TAG" "$DMG" --clobber
else
  gh release create "$TAG" "$DMG" --title "$TAG" --generate-notes
fi

printf '\n\033[32mdone: %s\033[0m\n\n' "$DMG"
