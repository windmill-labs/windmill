# Windmill's build guide

## Using Nix (Recommended)

Nix will manage all environment variables, packages and other configuration that you would usually do manually.

**Prerequisites**

- Install [Nix](https://github.com/DeterminateSystems/nix-installer).
- Install Docker.
- Optionally install [direnv](https://direnv.net/docs/installation.html).

That's it! You are ready to go.

> Using **direnv** is highly recommended, since it can load shell automatically based on your CWD. It also can give you hints.

### Development
```bash
# enter a dev shell containing all necessary packages. `direnv allow` if direnv is installed.
nix develop 
## or ignore if you have `direnv`

# Start db (if not started already)
sudo docker compose up db -d 

# run the frontend.
wm

# In an other shell:
#
nix develop 
## or ignore if you have `direnv`

cd backend
# You don't need to install anything extra. All dependencies are already in place!
cargo run --features all_languages 
```

The default proxy is setup to use the local backend: <http://localhost:8000>.

### wm-* Commands

Nix shell provides you with several helper commands prefixed with `wm-`

```bash
# Start minio server (implements S3)
wm-minio 
# Note: You will need access to EE private repo in order to compile, don't forget "enterprise" and "parquet" freatures as well. 

# Generate keys for local dev.
wm-minio-keys 
# Minio data as well as generated keys are stored in `backend/.minio-data`
```

You can read about all others commands individually in [flake.nix](../flake.nix). 

### Building Docker image locally

Sometimes it is important to build docker image for your branch locally. It is crucial part of testing, since local environment may differ from the containerized one.

That's why we provide `docker/dev.nu`. It is helper that can build images locally and execute them. 

it can build the image and run on local repository.

```
# Issue the build
docker/dev.nu up --features "python,static_frontend" docker/DockerfileNsjail --rebuild
# Will create and run `main__nsjail__python-static_frontend`
```

If you develop wasm parser for new language you can also pass --wasm-pkg <language> and it will include local parser to the image. For more information please see the script directly or run it with `--help` flag.

### dev.nu

In some places we have `dev.nu` files. They can help you developing and testing specific features. Their functionality depends on context, but you can get more info by running it with `--help` flag.

### Running on NixOS

It is recommended to use [nix-alien](https://github.com/thiagokokada/nix-alien) for running compiled windmill binaries. We need this because sometimes windmill may fetch unpatched binaries that are not compatible with NixOS.

Windmill use [nix flakes](https://nixos.wiki/wiki/Flakes):
```bash
nix run github:windmill-labs/windmill
```

## Traditional instructions

### Frontend only
In the `frontend/` directory:

- install the dependencies with `npm install` (or `pnpm install` or `yarn`)
- generate the windmill client:

```bash
# Install dependencies.
npm install # or pnpm or yarn or bun

# Generate windmill client.
npm run generate-backend-client
## on mac use
npm run generate-backend-client-mac

# Start dev server
npm run dev
```

The default proxy is setup to use the remote backend: <https://app.windmill.dev>.

You can configure another proxy to use like so:

```bash
REMOTE=http://127.0.0.1:8000 REMOTE_LSP=http://127.0.0.1:3001 npm run dev
```

### Use a Local backend

#### 1. Backend is run by docker

```bash
docker build . -t windmill
docker compose up db windmill_server windmill_worker
```

```bash
REMOTE=http://localhost REMOTE_LSP=http://localhost npm run dev
```

#### 2. Backend is run by docker, but built from the local source code

```bash
# Issue the build
docker/dev.nu up --features "python,static_frontend" docker/DockerfileNsjail --rebuild
# Will create and run `main__nsjail__python-static_frontend`
```

If you develop wasm parser for new language you can also pass --wasm-pkg <language> and it will include local parser to the image. For more information please see the script directly or run it with `--help` flag.

#### 3. Backend is run by cargo

**Prerequisites**

- Install Rust [as explained on the website](https://www.rust-lang.org/tools/install).
- Install llvm

  **On OSX:**

  ```bash
  brew install llvm gsed

  # make LLVM tools available on PATH
  echo 'export PATH="/opt/homebrew/opt/llvm/bin:$PATH"' >> ~/.zshrc

  # now, restart your shell. You should now have the `lld` binary on your PATH.
  ```

- To test that you have Rust and Cargo installed run `cargo --version`

**Known issue on M1 Mac while running `cargorun`**

- You may encounter `linking with cc failed` build time error.
- To solve this run:
  ```bash
  echo 'export RUSTFLAGS="-L/opt/homebrew/opt/libomp/lib"' >> ~/.zshrc
  source ~/.zshrc
  ```

In the root folder:

```bash
docker-compose up db
```

In the backend folder:

```bash
DATABASE_URL=postgres://postgres:changeme@127.0.0.1:5433/windmill?sslmode=disable cargo run
```

In the frontend folder:

```bash
REMOTE=http://127.0.0.1:8000 REMOTE_LSP=http://127.0.0.1:3001 npm run dev
```

### Formatting

This project uses [prettier](https://prettier.io/docs/en/install.html) and
[prettier-plugin-svelte](https://github.com/sveltejs/prettier-plugin-svelte), be
sure to install them and set up your editor to run prettier automatically before
you commit.

Recommended config for VS Code:

- [Prettier](https://marketplace.visualstudio.com/items?itemName=esbenp.prettier-vscode)
  for formatting
- [Svelte for VS Code](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode)
  for highlighting and Intellisense
- make sure that your VS Code `settings.json` has the following lines:

```json
"[svelte]": {
"editor.defaultFormatter": "esbenp.prettier-vscode"
}
```

- turn _format on save_ on

### Building frontend

The project is built with [SvelteKit](https://kit.svelte.dev/) and uses as output static files.
There are others adapters for sveltekit, but we use the static adapter.

To build the frontend as static assets, use:

```bash
NODE_OPTIONS=--max_old_space_size=8096 npm run build
```

The output is in the `build` folder.

The default build assume you serve every non static files as the 200.html file which is catchall. If you prefer a normal layout, you can use:

```
NOTCATCHALL=true npm run build
```

which will generate an index.html and allow you to serve the frontend with any static server.

Env variables used for build are set in .env file. See [https://vitejs.dev/guide/env-and-mode.html#env-files](https://vitejs.dev/guide/env-and-mode.html#env-files) for more details.

## Updating [`flake.nix`](../flake.nix)

```bash
nix flake update # update the lock file.
```

Some cargo dependencies use fixed git revisions, which are also fixed in `flake.nix`:
```nix
outputHashes = {
  "php-parser-rs-0.1.3" = "sha256-ZeI3KgUPmtjlRfq6eAYveqt8Ay35gwj6B9iOQRjQa9A=";
  # ...
};
```

When updating a revision, replace the incorrect `sha256` with `pkgs.lib.fakeHash`:
```diff
-            "php-parser-rs-0.1.3" = "sha256-ZeI3KgUPmtjlRfq6eAYveqt8Ay35gwj6B9iOQRjQa9A=";
+            "php-parser-rs-0.1.3" = pkgs.lib.fakeHash;
```

Then run `nix build .#windmill` and update with the correct sha.
