#!/bin/bash
set eou -pipefail

CANAME='windmill-root'

COUNTRY='FR'
STATE='Paris'
CITY='Paris'
ORGANIZATION='WindmillLabs'
ROOT_CA_CN='WindmillRootCA'
SERVER_CA_CN='caddy' # IMPORTANT: set this to the FQDN of the caddy server. Here in the docker compose stack, it will be caddy

echo "Generating RootCA key"
openssl genrsa -aes256 -out $CANAME.key 4096
echo "Generating RootCA certificate"
openssl req -x509 -new -nodes -key $CANAME.key -sha256 -days 1826 -out $CANAME.crt -subj "/CN=${ROOT_CA_CN}/C=${COUNTRY}/ST=${STATE}/L=${CITY}/O=${ORGANIZATION}"

CERTNAME=caddy
echo "Generating server certificate private key and cert signing request"
openssl req -new -nodes -out $CERTNAME.csr -newkey rsa:4096 -keyout $CERTNAME.key -subj "/CN=${SERVER_CA_CN}/C=${COUNTRY}/ST=${STATE}/L=${CITY}/O=${ORGANIZATION}"

echo "Generating server certificate"
cat > $CERTNAME.v3.ext << EOF
[v3_req]
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
subjectAltName = @alt_names
[alt_names]
DNS.1 = caddy
DNS.2 = localhost
EOF
# ^ HERE ^ in the above alt_names, feel free to add any alternate CN

openssl x509 -req -in $CERTNAME.csr -CA $CANAME.crt -CAkey $CANAME.key -CAcreateserial -out $CERTNAME.crt -days 1826 -sha256 -extensions v3_req -extfile $CERTNAME.v3.ext
