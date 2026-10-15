#!/bin/bash
# Check an installed Ruz.app: signature valid, designated requirement stable
# (certificate-anchored, not a cdhash), hardened runtime with the Calendar and
# Location entitlements, bundle keys present. Read-only.
set -euo pipefail
APP="${1:-/Applications/Ruz.app}"
fail=0

echo "== $APP"
codesign --verify --strict --verbose=2 "$APP"
DR=$(codesign -d -r- "$APP" 2>&1 | grep 'designated' || true)
echo "designated requirement: ${DR#*=> }"
if [[ "$DR" == *cdhash* ]]; then
  echo "FAIL: requirement is a cdhash — the grants will be lost on every rebuild." >&2
  fail=1
elif [[ "$DR" == *"certificate leaf"* || "$DR" == *"anchor"* ]]; then
  echo "ok: requirement is anchored to a certificate; grants survive rebuilds."
fi

INFO=$(codesign -dv "$APP" 2>&1)
if [[ "$INFO" == *"flags="*"(runtime)"* ]]; then
  echo "ok: hardened runtime"
  ENT=$(codesign -d --entitlements - --xml "$APP" 2>/dev/null || true)
  for e in calendars location; do
    if [[ "$ENT" == *"com.apple.security.personal-information.$e"* ]]; then
      echo "ok: $e entitlement"
    else
      echo "FAIL: missing com.apple.security.personal-information.$e — macOS will deny it silently." >&2
      fail=1
    fi
  done
fi

for key in CFBundleIdentifier LSUIElement CFBundleExecutable CFBundleShortVersionString; do
  printf '%-28s %s\n' "$key" "$(plutil -extract "$key" raw "$APP/Contents/Info.plist")"
done
exit $fail
