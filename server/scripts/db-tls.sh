#!/bin/sh
# Generate local TLS credentials under gitignored .workspace/, never in Git.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd -P)
tls="$root/.workspace/db-fixture-tls"
if [ -f "$tls/server.crt" ] && [ -f "$tls/server.key" ] && [ -f "$tls/ca.crt" ] && [ -f "$tls/wrong-ca.crt" ]; then exit 0; fi
mkdir -p "$root/.workspace"
scratch=$(mktemp -d "$root/.workspace/db-tls.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
umask 077
openssl req -x509 -newkey rsa:2048 -nodes -keyout "$scratch/ca.key" -out "$scratch/ca.crt" -days 3650 -subj '/CN=Sidereal fixture CA' >"$scratch/generation.log" 2>&1
openssl req -newkey rsa:2048 -nodes -keyout "$scratch/server.key" -out "$scratch/server.csr" -subj '/CN=localhost' >>"$scratch/generation.log" 2>&1
printf 'subjectAltName=DNS:localhost\nextendedKeyUsage=serverAuth\n' > "$scratch/server.ext"
openssl x509 -req -in "$scratch/server.csr" -CA "$scratch/ca.crt" -CAkey "$scratch/ca.key" -set_serial 1 -out "$scratch/server.crt" -days 3650 -extfile "$scratch/server.ext" >>"$scratch/generation.log" 2>&1
openssl req -x509 -newkey rsa:2048 -nodes -keyout "$scratch/wrong-ca.key" -out "$scratch/wrong-ca.crt" -days 3650 -subj '/CN=Untrusted fixture CA' >>"$scratch/generation.log" 2>&1
mkdir -p "$tls"
cp "$scratch/server.key" "$scratch/server.crt" "$scratch/ca.crt" "$scratch/wrong-ca.crt" "$tls/"
chmod 755 "$tls"
chmod 644 "$tls"/*.crt
# CA signing keys are discarded with scratch; the fixture only needs its server key.
