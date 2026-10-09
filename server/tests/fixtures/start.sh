#!/bin/sh
set -eu
# Public test key; PostgreSQL requires restrictive permissions on its copy.
install -o postgres -g postgres -m 600 /fixture/tls/server.key /var/lib/postgresql/server.key
exec docker-entrypoint.sh postgres -c ssl=on -c ssl_cert_file=/fixture/tls/server.crt -c ssl_key_file=/var/lib/postgresql/server.key
