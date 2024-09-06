# @deno/shim-deno

[`Deno` namespace](https://doc.deno.land/builtin/stable) shim for Node.js.

See
[PROGRESS.md](https://github.com/denoland/node_deno_shims/blob/main/packages/shim-deno/PROGRESS.md)

## Acknowledgements

Special thanks to the [@fromdeno](https://github.com/fromdeno) organization for
starting this project and for their contributions—specifically
[@wojpawlik](https://github.com/wojpawlik),
[@MKRhere](https://github.com/MKRhere), and
[@trgwii](https://github.com/trgwii).

## Contributing

### Updating Deno

1. Update local version.
1. In `/.github/workflows/ci.yml`, increase the deno version in the setup-deno
   action
1. Update version in `./tools/denolib.ts`
1. Go into `./third_party/deno` and update the submodule (there's probably
   better instructions):
   - `git fetch --tags`
   - `git checkout v1.x.x` -- replace with version
   - `git submodule update`
1. In this package ensure the following work and if not, fix any issues:
   - `npm run build`
   - `npm run test`
