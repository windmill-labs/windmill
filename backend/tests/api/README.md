# Api integration tests

You'll need deno to be installed and a windmill api endpoint.

By default the tests use the local environment endpoint <http://localhost:8000>. You can override it using `BASE_URL` environment variable.

By default the api authentication uses default local env user credentials. You can override api token using `WM_TOKEN` environment variable.

## Running the api tests

`deno test --allow-env --allow-net`
