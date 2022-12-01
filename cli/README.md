# Windmill CLI

A simple CLI allowing interactions with windmill from the command line.
![](./vhs/output/setup.gif)

## Installation

Install the `wmill` CLI tool using
`deno install --unstable -A https://deno.land/x/wmill/main.ts`.

Update to the latest version using `wmill upgrade`.

## Workspaces

To get started run `wmill workspace add` or use the instructions from the
workspace settings.

## Running Flows & Scripts

Run a script or flow using `wmill flow/script run u/username/path/to/script` and
pass any inputs using `--input/-i <name>=<value>` curl-style syntax using
`-i @-` for stdin or `-i @<filename>` is also supported

Flow Steps and Logs will be streamed during execution automatically.
![](./vhs/output/run-flow.gif)

## Pushing Resources, Scripts & More

The CLI can push specifications to a windmill instance. See the
[examples/](./examples/) folder for formats.

### Pushing a folder

You can push all files in a folder at once using `wmill push` Files MUST be
named resource_name.\<type\>.json. They will be pushed to the remote path they
are in, for example the file `u/admin/fib/fib.script.json` will be pushed as a
script to u/admin/fib/fib.

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
