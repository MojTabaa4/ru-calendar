#!/bin/bash
# Create a self-signed code-signing certificate for local builds, once.
#
# Why: macOS ties Ruz's Calendar and Location grants, and the Keychain items
# holding its Google sign-ins, to the code signature. An ad-hoc signature's
# designated requirement is the binary's cdhash, which changes on every build,
# so each rebuild asks for the permissions again and the Keychain prompts for
# its own items. A stable certificate gives a requirement of
# `identifier … and certificate leaf = H"…"`, which survives rebuilds.
#
# SAFETY — this script never touches any keychain. No `security` commands at
# all: no search-list edits, nothing imported into the login keychain, no trust
# settings changed. (`security list-keychains -s …` *replaces* the search list;
# run carelessly it evicts login.keychain-db and signs you out of every app that
# keeps secrets there.) The key and certificate are plain PEM files, used
# directly by `rcodesign` — see scripts/build-app.sh.
set -euo pipefail

DIR="$HOME/Library/Application Support/ai.intelcad.ruz/dev-signing"

if [[ -f "$DIR/cert.pem" && -f "$DIR/key.pem" ]]; then
  echo "Signing certificate already exists in: $DIR"
  echo "Not overwriting it: a new certificate would reset the Calendar and Location grants."
  printf 'Leaf SHA-1: '; openssl x509 -in "$DIR/cert.pem" -outform DER | shasum -a 1 | cut -d' ' -f1
  exit 0
fi

mkdir -p "$DIR"
chmod 700 "$DIR"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# A config file (rather than -addext) works with both OpenSSL and macOS's LibreSSL.
cat > "$TMP/cert.cnf" <<'CNF'
[req]
distinguished_name = dn
x509_extensions = ext
prompt = no
[dn]
CN = Ruz Dev (local)
[ext]
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
basicConstraints = critical, CA:false
subjectKeyIdentifier = hash
CNF

openssl req -x509 -newkey rsa:2048 -days 3650 -nodes \
  -keyout "$DIR/key.pem" -out "$DIR/cert.pem" -config "$TMP/cert.cnf" 2>/dev/null
chmod 600 "$DIR/key.pem" "$DIR/cert.pem"

echo "Created a local code-signing certificate in: $DIR"
printf 'Leaf SHA-1: '; openssl x509 -in "$DIR/cert.pem" -outform DER | shasum -a 1 | cut -d' ' -f1
echo "Keep key.pem private: anything signed with it inherits Ruz's grants and Keychain access."
