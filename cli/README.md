# Windmill CLI

A simple CLI allowing interactions with windmill from the command line.
<video src='./vhs/output/setup.mp4' width=600 />

## Installation

Install the `wmill` CLI tool using `deno install --unstable -A https://deno.land/x/wmill/main.ts`.

Update to the latest version using `wmill upgrade`.

## Setup

Setup the CLI by running `wmill setup`. This will guide you through the setup process easily.

## Running Flows & Scripts

Run a script or flow using `wmill flow/script run u/username/path/to/script` and pass any inputs using `--input/-i <name>=<value>` curl-style syntax using `-i @-` for stdin or `-i @<filename>` is also supported

Flow Steps and Logs will be streamed during execution automatically.
<video src='./vhs/output/run-flow.mp4' width=600 />

## Pushing Resources, Scripts & More

The CLI can push specifications to a windmill instance. See the [examples/](./examples/) folder for formats.

### Pushing a folder

You can push all files in a folder at once using `wmill push`
Files MUST be named resource_name.\<type\>.json. They will be pushed to the remote path they are in, for example the file `u/admin/fib/fib.script.json` will be pushed as a script to u/admin/fib/fib.

### Pushing individual files

You can push individual resources using `wmill <type> push <file_name> \<remote_name\>`. This does not require a special folder layout or file name, as this is given at runtime.

## Listing

All commands support listing by just not providing a subcommand, ie `wmill script` will result in a list of scripts. Some allow additional options, learn about this by specifying `--help`.

## User Management

You can add & remove users via `wmill user add/remove`, and list them using `wmill user`

## Login

Logging in using `wmill login` or the setup will save a token to your local computer, into `~/.config/windmill/<hash>/token` (or `C:\Users\<username>\AppData\Roaming\windmill\<hash>\token` on windows).
This is inherently unsafe, so do not log into the CLI on untrusted devices.

## Managing Remotes

Advanced users may use multiple remotes at once, which the CLI supports using `wmill remote`.

Add remotes using `wmill remote add <name> <base_url>` & Remove them using `wmill remote remove <name>`.

You can use a remote by either setting it as default using `wmill remote set-default <name>` or by overriding the remote using `--remote <name>` on any command.

If you don't want to save the URL locally, you can always override the command using `--base-url <base_url>` on any command.

### Login on multiple remotes

You will have to login on each remote, either do this using `wmill login` or override the token/credentials on each command using `--token <token>`/`--username <username> --password <password>`.

## Completion

The CLI comes with completions out of the box via `wmill completions <shell>`. (Via [cliffy](https://cliffy.io/))

### Bash

To enable bash completions add the following line to your `~/.bashrc`:

```
source <(wmill completions bash)
```

### Fish

To enable fish completions add the following line to your `~/.config/fish/config.fish`:

```
source (wmill completions fish | psub)
```

### Zsh

To enable zsh completions add the following line to your `~/.zshrc`:

```
source <(wmill completions zsh)
```
