# Developing

## Starting the Development Server

Once you've created a project and installed dependencies with `npm install` (or
`pnpm install` or `yarn`), start a development server:

```bash
npm run dev
```

To have the whole stack backing your dev environment, 3 solutions:

### 1. Backend is run by docker

In the root folder:

```bash
docker build . -t windmill
docker-compose up db server
```

### 2. Backend is run by cargo

**Prerequisites**

- Install Rust [as explained on the website](https://www.rust-lang.org/tools/install).
- Install llvm 

  **on OSX:**
  ```bash
  brew install llvm caddy gsed
  
  # make LLVM tools available on PATH
  echo 'export PATH="/opt/homebrew/opt/llvm/bin:$PATH"' >> ~/.zshrc
  
  # now, restart your shell. You should now have the `lld` binary on your PATH.
  ```
  
**Do a Frontend Build**

In order to run the backend, you need to have a frontend build inside `frontend/build/`.

Otherwise, `cargo run` will break.

So, in the frontend folder, run:

```bash
# !!! on OSX, you are not allowed to use the system SED, but you need to use GNU SED.
# !!! thus, in `frontend/package.json`, replace all `sed` occurences with `gsed`.

# prerequisite for build
npm run generate-backend-client

npm run build
# now, you'll have a `frontend/build` folder.
```

In the root folder:

```bash
docker-compose up db
```

In the backend folder:

```bash
DATABASE_URL=postgres://postgres:changeme@127.0.0.1:5433/windmill?sslmode=disable cargo run
```

You can now access [http://127.0.0.1:8000](http://127.0.0.1:8000).

### In both cases

In the frontend folder:

```bash
sudo caddy run --config ./Caddyfile
```

(sudo is required to bind port 80 and 443)

and then go to <http://localhost>

### Backend is run by remote!

```bash
sudo caddy run --config ./CaddyfileRemote
```

and then go to <http://localhost>

## Building

```bash
npm run build
```

## Generating the backend client automatically

```bash
npm run generate-backend-client
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
