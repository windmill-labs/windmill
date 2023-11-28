Private NPM registry with self-signed certificates
==================================================

Setup a private NPM registry with self-signed certificates.

### Setup

1. Generate self signed certificates (or bring your own)
```bash
cd certs
# feel free to read the file anc change the 
./generate_certs.sh
# choose a password for the RootCA key. You'll need to input it for pretty much all following steps
```
At the end of the script, you should have multiple files in the `certs/` folder. The most important ones are:
- `windmill-root.key` (Root CA private key)
- `windmill-root.crt` (Root CA certificate)
- `npm_registry.key` (NPM registry server private key)
- `npm_registry.crt` (NPM registry server certificate)

2. Start the docker compose stack

```bash
docker compose up -d
```

This will start the private NPM registry, as well as a minimal Windmill stack composed of just one Windmill server/worker and the associated database. 

For the latter, we invite you to refer to the latest [docker compose](/docker-compose.yml) at the root of this repository to setup a more evolved Windmill stack.

For the former, it's using [Verdaccio](https://verdaccio.org/) as an easy-to-deploy NPM registry. We bring you attention to the fact that in addition to the config 
file in `./verdaccio_conf/config.yaml`, we had to set both `VERDACCIO_PROTOCOL` and `VERDACCIO_PUBLIC_URL` in docker compose. See the official Verdaccio 
documentation for more info on this.

3. Upload the custom `helloworld` package to the private NPM registry

```
cd helloworld_package
# you need to first create a registry user
# you might need to run `npm config set strict-ssl false` if the below fails. If you do, then change it back to `true` after running the 2 commands
npm adduser --registry https://0.0.0.0:4873/
npm publish --registry https://0.0.0.0:4873/
```

4. Pull the custom NPM package from a deno script in Windmill

Go to Windmill at `http://localhost:8000`. Create a simple deno script:
```ts
import * as testpackage from "npm:@windmill/helloworld@0.0.1"

export async function main() {
  console.log(testpackage.sayHello("Windmill"))
}
```
and execute it. It should return successfully with:
```
Hello Windmill
```


### Remarks

1. `DENO_TLS_CA_STORE` VS `DENO_CERT`
Both works. `DENO_CERT` is better b/c you just have to set it to the path of the trusted Root CA certificate, and deno will trust this certificate.
When using `DENO_TLS_CA_STORE=system`, you _have to_ make the server trust the custom Root CA certificate with the following commands:
```bash
# in the windmill-server container:
cp /custom-certs/windmill-root.crt /usr/local/share/ca-certificates/
update-ca-certificates # as root
# the output should tell (among other things): " ... 1 added, 0 removed; done. ..."
```

2. Running deno scripts manually in the Windmill container
This could be useful for debugging purposes.

```bash
# log into the Windmill Server container
docker exec -it <WINDMILL_SERVER_CONTAINER> /bin/bash

# Go into Deno REPL
deno

# try to import the package
import * as testpackage from "npm:@windmill/helloworld@0.0.1"
> undefined

# if the above returns undefined, there's a good chance it's working. If you want to double check:
testpackage.sayHello("Windmill")
> Hello Windmill

# potentially inspect the env var available to DENO. Is you used DENO_CERT in the docker compose, check its value:
console.log(Deno.env.get("DENO_CERT"))
> /custom-certs/windmill-root.crt
```
