#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
interop_dir=$(mktemp -d)
trap 'rm -rf "$interop_dir"' EXIT
openssl req -x509 -newkey rsa:2048 -nodes -keyout "$interop_dir/key.pem" -out "$interop_dir/cert.pem" -days 2 -subj '/CN=Bridge Interoperability Test/O=Disposable Test Certificate' 2>/dev/null
openssl x509 -in "$interop_dir/cert.pem" -outform DER -out "$interop_dir/cert.der"
cat > "$interop_dir/tsa.cnf" <<EOF
[req]
distinguished_name = dn
x509_extensions = tsa_ext
prompt = no
[dn]
CN = Disposable Timestamp Test
[ca_ext]
basicConstraints = critical,CA:TRUE
keyUsage = critical,keyCertSign,cRLSign
[tsa_ext]
basicConstraints = critical,CA:FALSE
keyUsage = critical,digitalSignature
extendedKeyUsage = critical,timeStamping
[tsa]
default_tsa = tsa_config
[tsa_config]
serial = $interop_dir/serial
signer_cert = $interop_dir/tsa.pem
signer_key = $interop_dir/tsa-key.pem
certs = $interop_dir/ca.pem
signer_digest = sha256
default_policy = 1.2.3.4.1
digests = sha1,sha256
accuracy = secs:1
ordering = no
tsa_name = yes
ess_cert_id_chain = no
EOF
openssl req -new -x509 -newkey rsa:2048 -nodes -keyout "$interop_dir/ca-key.pem" -out "$interop_dir/ca.pem" -days 2 -config "$interop_dir/tsa.cnf" -extensions ca_ext -subj '/CN=Disposable Timestamp Root' 2>/dev/null
openssl req -new -newkey rsa:2048 -nodes -keyout "$interop_dir/tsa-key.pem" -out "$interop_dir/tsa.csr" -config "$interop_dir/tsa.cnf" 2>/dev/null
openssl x509 -req -in "$interop_dir/tsa.csr" -CA "$interop_dir/ca.pem" -CAkey "$interop_dir/ca-key.pem" -CAcreateserial -out "$interop_dir/tsa.pem" -days 2 -extfile "$interop_dir/tsa.cnf" -extensions tsa_ext 2>/dev/null
printf '%s\n' '01' > "$interop_dir/serial"
BRIDGE_TEST_TSA_CONFIG="$interop_dir/tsa.cnf" BRIDGE_INTEROP_DIR="$interop_dir" cargo test --manifest-path src-tauri/Cargo.toml xades::interoperability -- --nocapture
java scripts/interop/Verify.java "$interop_dir"

python3 scripts/interop/verify_timestamp.py "$interop_dir"
