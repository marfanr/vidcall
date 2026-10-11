#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"

CERT_DIR="certs"
ENV_FILE=".env"
CA_DAYS=3650
SERVER_DAYS=825
SAN="DNS:localhost,DNS:postgres,IP:127.0.0.1"

FORCE=0
[[ "${1:-}" == "--force" ]] && FORCE=1

command -v openssl >/dev/null || { echo "openssl not found" >&2; exit 1; }

mkdir -p "$CERT_DIR"
cd "$CERT_DIR"

if [[ $FORCE -eq 1 || ! -f ca.key || ! -f ca.crt ]]; then
  echo "==> Generating CA"
  rm -f ca.key ca.crt ca.srl
  openssl genrsa -out ca.key 4096 2>/dev/null
  openssl req -x509 -new -key ca.key -sha256 -days "$CA_DAYS" \
    -subj "/CN=vidcall-dev-ca" \
    -addext "basicConstraints=critical,CA:TRUE" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" \
    -out ca.crt
  FORCE=1
else
  echo "==> CA already exists, reusing"
fi

if [[ $FORCE -eq 1 || ! -f server.key || ! -f server.crt ]]; then
  echo "==> Generating server cert (SAN: $SAN)"
  rm -f server.key server.crt server.csr san.cnf
  openssl genrsa -out server.key 2048 2>/dev/null
  openssl req -new -key server.key -subj "/CN=postgres" -out server.csr

  cat > san.cnf <<EOF
subjectAltName=$SAN
extendedKeyUsage=serverAuth
basicConstraints=CA:FALSE
keyUsage=digitalSignature,keyEncipherment
EOF

  openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial \
    -out server.crt -days "$SERVER_DAYS" -sha256 -extfile san.cnf 2>/dev/null
  rm -f server.csr san.cnf
else
  echo "==> Server cert already exists, reusing"
fi

openssl verify -CAfile ca.crt server.crt >/dev/null
openssl x509 -in server.crt -noout -ext subjectAltName | tail -n +2

chmod 600 ca.key server.key
chmod 644 ca.crt server.crt
cd ..

touch "$ENV_FILE"
tmp="$(mktemp)"
grep -vE '^(SSL_CERTIFICATE|SSL_PRIVATE_KEY|PG_CA_CERT)=' "$ENV_FILE" > "$tmp" || true

{
  cat "$tmp"
  printf 'SSL_CERTIFICATE="%s"\n' "$(awk '{printf "%s\\n",$0}' "$CERT_DIR/server.crt")"
  printf 'SSL_PRIVATE_KEY="%s"\n' "$(awk '{printf "%s\\n",$0}' "$CERT_DIR/server.key")"
  printf 'PG_CA_CERT=%s\n' "$CERT_DIR/ca.crt"
} > "$ENV_FILE"
rm -f "$tmp"
chmod 600 "$ENV_FILE"

touch .gitignore
for entry in "certs/" ".env"; do
  grep -qxF "$entry" .gitignore || echo "$entry" >> .gitignore
done

echo
echo "Done. SSL entries in $ENV_FILE: $(grep -cE '^(SSL_CERTIFICATE|SSL_PRIVATE_KEY)=' "$ENV_FILE") (expected 2)"
echo "Next steps:"
echo "  docker compose up -d --force-recreate postgres"
echo "  openssl s_client -connect localhost:5432 -starttls postgres -CAfile $CERT_DIR/ca.crt </dev/null 2>&1 | grep -E 'Verif'"