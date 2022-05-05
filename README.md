<p align="center">
  <a href="https://alpha.windmill.dev"><img src="./windmill.svg" alt="windmill.dev"></a>
</p>
<p align="center">
    <em>Windmill.dev is an OSS developer platform to quickly build production-grade multi-steps automations and internal apps from minimal Python and Typescript scripts.</em>
</p>
<p align="center">
<a href="https://pypi.org/project/wmill" target="_blank">
    <img src="https://img.shields.io/pypi/v/wmill?color=%2334D058&label=pypi%20package" alt="Package version">
</a>
<a href="https://discord.gg/V7PM2YHsPB" target="_blank">
  <img src="https://discordapp.com/api/guilds/930051556043276338/widget.png" alt="Discord Shield"/>
</a>
</p>

---

**Join the alpha (personal workspaces are free forever)**:
<https://alpha.windmill.dev>

**Documentation**: <https://docs.windmill.dev>

**Discord**: <https://discord.gg/V7PM2YHsPB>

**We are hiring**: Software Engineers, DevOps, Solutions Engineers, Growth:
<https://docs.windmill.dev/hiring>

You can show your support for the project by starring this repo.

---

# Windmill

![Windmill](./windmill.webp)

Windmill is fully open-sourced:

- community parts and python-client are Apache 2.0
- backend, frontend and everything else under AGPLv3.

## Stack

- postgres as the database
- backend in Rust with the follwing highly-available and horizontally scalable
  architecture:
  - stateless API backend
  - workers that pull jobs from a queue
- frontend in svelte
- scripts executions are sandboxed using google's nsjail
- javascript runtime is deno_core rust library (which itself uses the rusty_v8
  and hence V8 underneath)
- typescript runtime is deno
- python runtime is python3

### Developent stack

- caddy is the reverse proxy + handle https

## How to self-host

Complete instructions coming soon

## Copyright

2021 [Ruben Fiszel](https://github.com/rubenfiszel)

### Acknowledgement

This project is inspired from a previous project called
[Delightool](https://github.com/windmill-labs/delightool-legacy) which was also
led by [Ruben](https://github.com/rubenfiszel) but its frontend was realized
with large contribution from [Malo Marrec](https://github.com/malomarrec).
Windmill is a child of Delightool but entirely distinct and realized with Malo's
blessing.
