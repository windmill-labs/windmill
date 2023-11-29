Private package registry with self-signed certificates
==================================================

Setup a private NPM registry and Pypi server behind a revers proxy (Caddy) exposing an HTTPS endpoint with self signed certificates

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
- `caddy.key` (caddy server private key)
- `caddy.crt` (caddy registry server certificate)

2. Start the docker compose stack

```bash
docker compose up -d
```

This will start the private NPM registry, the private Pypi server and Caddy as a reverse proxy sitting in front of the two. Separately, for the purpose of proving an end to end setup, it will also start minimal Windmill stack composed of just one Windmill server/worker and the associated database. 

For a more complete Windmill setup, we invite you to refer to the latest [docker compose](/docker-compose.yml) at the root of this repository to setup a more evolved Windmill stack.

For the private registries/repositories, it's using [Verdaccio](https://verdaccio.org/) as an easy-to-deploy NPM registry and [pypiserver](https://pypi.org/project/pypiserver/) for Python.

## Deno pulling package from private NPM registry

Upload the custom `helloworld_npm_package` package to the private NPM registry and use it in a Windmill script

```bash
cd helloworld_npm_package
# you need to first create a registry user
# you might need to run `npm config set strict-ssl false` if the below fails. If you do, then change it back to `true` after running the 2 commands
npm adduser --registry https://localhost/npm/
npm publish --registry https://localhost/npm/
```

Go to Windmill at `http://localhost:8000`. Create a simple Deno script:
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

Note: Native fetches to HTTPS endpoints with self signed certificates also takes into account the DENO_CERT environment variable

Go to Windmill and create a new script of type "REST" with the following content:
```ts
export async function main() {
  const res = await fetch("https://caddy/static/helloworld.json", {
    headers: { "Content-Type": "application/json" },
  });
  return res.json();
}
```

It will fetch a static json file from Caddy exposed behind its HTTPS endpoint with self signed certificate.

## Python pulling package from private NPM registry

Upload the custom `helloworld_python_module` python module to the private Pypi server

```bash
cd helloworld_python_module
python setup.py sdist
# using twine to upload the module here, get it with `pip install twine`
twine upload --repository-url https://localhost/ dist/* --cert ../certs/windmill-root.crt
# no username and password, just press enter. For the purpose of the demo we're running pypiserver completely unauthenticated
```
You can check that the package is uploaded by visiting [https://localhost/simple](https://localhost/simple).

Go to Windmill at `http://localhost:8000`. Create a simple Python script:
```python
#requirements:
#windmill-helloworld==0.0.1
import windmill_helloworld

def main():
    print(windmill_helloworld.say_hello("Windmill"))
```
and execute it. It should return successfully with:
```
Hello Windmill
```

### MISC

1. `DENO_TLS_CA_STORE` VS `DENO_CERT`
Both works. `DENO_CERT` is better b/c you just have to set it to the path of the trusted Root CA certificate, and deno will trust this certificate.
When using `DENO_TLS_CA_STORE=system`, you _have to_ make the server trust the custom Root CA certificate with the following commands:
```bash
# in the windmill-server container:
cp /custom-certs/windmill-root.crt /usr/local/share/ca-certificates/
update-ca-certificates # as root
# the output should tell (among other things): " ... 1 added, 0 removed; done. ..."
```

2. Running Deno scripts manually in the Windmill container
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

3. Manually testing Pypi private server integration
Can be useful to debug as well:

```bash
# install the package
pip3 install --cert ../certs/windmill-root.crt -i https://localhost/simple/ windmill-helloworld

# go to python CLI and try to import it and use it
python
>>> import windmill_helloworld
>>> windmill_helloworld.say_hello("world")
'Hello world'
```
