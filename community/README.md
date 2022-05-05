# Windmill community scripts

**Contributions welcome!**

The structure of the public templates are as follows:

- `starter/`:
  - `resource_types/`: The resource types as
    [jsonschema](https://json-schema.org/)
  - `resources/u/bot/<foo>.json`: The resources
  - `scripts/u/bot/<foo>.py`: The scripts code
  - `scripts/u/bot/<foo>.json`: The corresponding script metadata
  - `flows/u/bot/<foo>.json`: The flow entire definition

On merge to the main branch, the changes are automatically pushed to the
`starter` workspace of windmill.dev using the github action:
[windmill-gh-action-deploy](https://github.com/windmill-labs/windmill-gh-action-deploy).

Any element in the `starter` workspace is available read-only in every
workspace. Whenever an element is updated in `starter`, it is updated in every
workspaces, not just new ones.

This repo serves also as an example of how to setup a repo to back and
automatically deploy scripts and resources to your workspace using
[windmill-gh-action-deploy](https://github.com/windmill-labs/windmill-gh-action-deploy).
Indeed, it is sufficient to use the same folder layout and setup an action
similar to the one
[here](https://github.com/windmill-labs/windmill/blob/main/.github/workflows/deploy_to_windmill.yml).
