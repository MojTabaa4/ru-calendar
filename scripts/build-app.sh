#!/bin/bash
# Build, bundle, sign and install Ruz.app — keeping its macOS permissions.
#
#   scripts/build-app.sh               # release build → /Applications/Ruz.app, relaunched
#   scripts/build-app.sh --no-launch   # install, but leave it stopped
#   RUZ_INSTALL=~/Applications/Ruz.app scripts/build-app.sh
#
# The Calendar and Location grants, and Keychain access to the Google sign-ins,
# survive because the signature does: the bundle is signed with the local
# certificate from scripts/make-cert.sh (created on first run), and its
# designated requirement is pinned to that certificate rather than to the
# binary's hash. macOS's own `codesign` cannot use a certificate that lives
# outside the keychain search list, so signing goes through `rcodesign`, which
# reads the PEM files directly — no keychain is ever read or written.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
APP_NAME=Ruz
BUNDLE_ID=ai.intelcad.ruz
EXE=ruz
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
OUT="$TARGET_DIR/release/bundle/macos/$APP_NAME.app"
INSTALL="${RUZ_INSTALL:-/Applications/$APP_NAME.app}"
SIGN_DIR="$HOME/Library/Application Support/$BUNDLE_ID/dev-signing"
ENTITLEMENTS="$ROOT/src-tauri/Entitlements.plist"
LAUNCH=1
[[ "${1:-}" == "--no-launch" ]] && LAUNCH=0

# rsync --delete below must only ever target an .app bundle.
case "$INSTALL" in
  */*.app) ;;
  *) echo "refusing to install to '$INSTALL': must be a path ending in .app" >&2; exit 1 ;;
esac

# The ruz CLI shares the executable name, so running copies of the app are
# found by their path inside a bundle, never by name alone.
APP_PROCESS="/$APP_NAME\.app/Contents/MacOS/$EXE( |$)"

# --- Tools ------------------------------------------------------------------
RCODESIGN=$(command -v rcodesign || true)
if [[ -z "$RCODESIGN" ]]; then
  echo "rcodesign is missing. Install it once with:" >&2
  echo "    cargo install apple-codesign --locked --bin rcodesign" >&2
  exit 1
fi
if ! command -v bun >/dev/null; then
  echo "bun is missing; see https://bun.sh" >&2
  exit 1
fi
if [[ ! -f "$SIGN_DIR/cert.pem" || ! -f "$SIGN_DIR/key.pem" ]]; then
  echo "==> no signing certificate yet; creating one"
  "$ROOT/scripts/make-cert.sh"
fi

# --- Build and bundle -------------------------------------------------------
# Tauri owns the bundle layout and the Info.plist (tauri.conf.json, merged with
# src-tauri/Info.plist), so there is one definition of both. Its own ad-hoc
# signature is replaced below.
[[ -d "$ROOT/node_modules" ]] || (cd "$ROOT" && bun install --frozen-lockfile)
echo "==> tauri build --bundles app"
(cd "$ROOT" && bun run tauri build --bundles app)

# --- Sign -------------------------------------------------------------------
SHA1=$(openssl x509 -in "$SIGN_DIR/cert.pem" -outform DER | shasum -a 1 | cut -d' ' -f1)
REQ="$TARGET_DIR/ruz-designated-requirement.bin"
# Anchor the requirement to our certificate, never to the cdhash: this line is
# the whole reason the grants survive a rebuild.
csreq -r "=identifier \"$BUNDLE_ID\" and certificate leaf = H\"$SHA1\"" -b "$REQ"
echo "==> signing with the local certificate (leaf $SHA1)"
"$RCODESIGN" sign \
  --pem-file "$SIGN_DIR/key.pem" --pem-file "$SIGN_DIR/cert.pem" \
  --code-requirements-file "$REQ" \
  --code-signature-flags runtime \
  --entitlements-xml-file "$ENTITLEMENTS" \
  --timestamp-url none \
  "$OUT" >/dev/null
codesign --verify --strict "$OUT"

# --- Install ----------------------------------------------------------------
# Crossing over from an ad-hoc install is the one rebuild that still costs a
# re-grant: the stored grants name the old binary's hash. Say so up front.
if [[ -d "$INSTALL" ]] && codesign -d -r- "$INSTALL" 2>&1 | grep -q 'designated => cdhash'; then
  CROSSOVER=1
else
  CROSSOVER=0
fi

# Ruz is single-instance: a copy still running would swallow the new launch.
if pgrep -f "$APP_PROCESS" >/dev/null; then
  echo "==> quitting the running $APP_NAME"
  pkill -f "$APP_PROCESS" || true
  for _ in {1..20}; do pgrep -f "$APP_PROCESS" >/dev/null || break; sleep 0.25; done
  pkill -9 -f "$APP_PROCESS" 2>/dev/null || true
fi
if pgrep -f "/target/(debug|release)/$EXE( |$)" >/dev/null; then
  echo "warning: a \`tauri dev\` copy of $APP_NAME is running; quit it, or the installed app" >&2
  echo "         will hand its launch over to that copy and exit." >&2
fi

echo "==> installing to $INSTALL"
mkdir -p "$(dirname "$INSTALL")"
rsync -a --delete "$OUT/" "$INSTALL/"
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$INSTALL" || true

if [[ $CROSSOVER == 1 ]]; then
  cat >&2 <<NOTE

ONE-TIME STEP: the previous install was ad-hoc signed, so its grants are tied to
that old binary and do not carry over. macOS will ask for Calendar and Location
again; if it doesn't, reset them with
    tccutil reset Calendar $BUNDLE_ID && tccutil reset Location $BUNDLE_ID
From now on every rebuild keeps them.
NOTE
fi

if [[ $LAUNCH == 1 ]]; then
  open "$INSTALL"
  echo "==> launched $INSTALL"
else
  echo "==> done. Launch with:  open \"$INSTALL\""
fi
echo "    Always launch from that path: macOS records it with the grants."
