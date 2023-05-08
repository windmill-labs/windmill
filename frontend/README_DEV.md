# Developing

- install the dependencies with `npm install` (or `pnpm install` or `yarn`)
- generate the windmill client:

```bash
npm run generate-backend-client
## on mac use
npm run generate-backend-client-mac
```

Once the dependencies are installed, just start the dev server:

```bash
npm run dev
```

The default proxy is setup to use the remote backend: <https://app.windmill.dev>.

You can configure another proxy to use like so:

```bash
REMOTE=http://127.0.0.1:8000 REMOTE_LSP=http://127.0.0.1:3001 npm run dev
```

## Use a Local backend

### 1. Backend is run by docker

```bash
docker build . -t windmill
docker compose up db windmill_server windmill_worker
```

```bash
REMOTE=http://localhost REMOTE_LSP=http://localhost npm run dev
```

### 2. Backend is run by cargo

**Prerequisites**

- Install Rust [as explained on the website](https://www.rust-lang.org/tools/install).
- Install llvm

  **On OSX:**

  ```bash
  brew install llvm caddy gsed

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

## Building

```bash
NODE_OPTIONS=--max_old_space_size=8096 npm run build
```

## Formatting

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

## Building

The project is built with [SvelteKit](https://kit.svelte.dev/) and uses as output static files.
There are others adapters for sveltekit, but we use the static adapter.

To build the frontend as static assets, use:

```
npm run build
```

The output is in the `build` folder.

The default build assume you serve every non static files as the 200.html file which is catchall. If you prefer a normal layout, you can use:

```
NOTCATCHALL=true npm run build
```

which will generate an index.html and allow you to serve the frontend with any static server.

Env variables used for build are set in .env file. See [https://vitejs.dev/guide/env-and-mode.html#env-files](https://vitejs.dev/guide/env-and-mode.html#env-files) for more details.
