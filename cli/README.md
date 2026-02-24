# Windmill CLI

A simple CLI allowing interactions with windmill from the command line.
![](./vhs/output/setup.gif)

You can find more information in
[Windmill Docs](https://www.windmill.dev/docs/advanced/cli)

## Installation

Install the `wmill` CLI tool using `npm install -g windmill-cli`.

Update to the latest version using `wmill upgrade`.

## Workspaces

To get started run `wmill workspace add` or use the instructions from the
workspace settings.

## Running Flows & Scripts

Run a script or flow using `wmill flow/script run u/username/path/to/script` and
pass any inputs using `--data` + Inputs specified as a JSON string or a file
using `@ <filename>` or stdin using @-.

Curl-style syntax using `-d @-` for stdin or `-d @<filename>` is also supported.

Flow Steps and Logs will be streamed during execution automatically.

![CLI input example](./vhs/output/cli_inputs_example.png)

## Pushing Resources, Scripts & More

The CLI can push specifications to a windmill instance. See the
[examples/](./examples/) folder for formats.

## Switch to a different workspace

```
wmill workspace switch <workspace_name>
```

## Sync a workspace

### Pull

```
wmill sync pull
```

### Push

```
wmill sync push
```

We recommend using the --yaml option to use yaml instead of json as the encoding
format. Yaml will be made the default soon.

### Pushing individual files

You can push individual resources using
`wmill <type> push <file_name> \<remote_name\>`. This does not require a special
folder layout or file name, as this is given at runtime.

## Listing

All commands support listing by just not providing a subcommand, ie
`wmill script` will result in a list of scripts. Some allow additional options,
learn about this by specifying `--help`.

## User Management

You can add & remove users via `wmill user add/remove`, and list them using
`wmill user`

## Pulling

You can pull the entire workspace using `wmill pull`

## Completion

The CLI comes with completions out of the box via `wmill completions <shell>`.
(Via [cliffy](https://cliffy.io/))

### Bash

To enable bash completions add the following line to your `~/.bashrc`:

```
source <(wmill completions bash)
```

### Fish

To enable fish completions add the following line to your
`~/.config/fish/config.fish`:

```
source (wmill completions fish | psub)
```

### Zsh

To enable zsh completions add the following line to your `~/.zshrc`:

```
source <(wmill completions zsh)
```

## Development

### Testing with a local `windmill-yaml-validator`

To test local changes to the validator before publishing, use `npm link`:

```bash
# In windmill-yaml-validator/
npm run build
npm link

# In cli/
npm link windmill-yaml-validator
```

### Running Tests

**Prerequisites:**
- PostgreSQL running locally (default: `postgres://postgres:changeme@localhost:5432`)
- Rust toolchain installed

**Run tests locally (full features):**

```bash
bun test test/
```

**Run tests in CI mode (minimal features, skips EE tests):**

```bash
CI_MINIMAL_FEATURES=true bun test test/
```

| Variable | Description |
|----------|-------------|
| `CI_MINIMAL_FEATURES` | Set to `true` to skip EE-dependent tests |
| `DATABASE_URL` | PostgreSQL connection string |
| `EE_LICENSE_KEY` | Enterprise license key for EE features |
