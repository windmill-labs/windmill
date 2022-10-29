# Windmill CLI

A simple CLI allowing interactions with windmill from the command line.

## Login

Logging in will save a token to your local computer, into `~/.config/windmill/<hash>/token` (or `C:\Users\<username>\AppData\Roaming\windmill\<hash>\token` on windows).
This is inherently unsafe, so do not log into the CLI on untrusted devices.

## Completion

The CLI comes with completions out of the box via `windmill completions <shell>`. (Via [cliffy](https://cliffy.io/))

### Bash

To enable bash completions add the following line to your `~/.bashrc`:

```
source <(windmill completions bash)
```

### Fish

To enable fish completions add the following line to your `~/.config/fish/config.fish`:

```
source (windmill completions fish | psub)
```

### Zsh

To enable zsh completions add the following line to your `~/.zshrc`:

```
source <(windmill completions zsh)
```
