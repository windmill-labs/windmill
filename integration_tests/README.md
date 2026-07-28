Windmill integration tests
==========================

Integration that are run on every push to the main branch (excluding tags)

The concept is the following:
1. It pulls the latest published image with the version from (version.txt)[../version.txt] and deploys a Windmill stack using the (docker-compose.yml)[docker-compose.yml]
2. It runs the tests using python testing framework (`python -m unittest -v test`). All test classes exported in (__init__.py)[./test/__init__py] will be run
3. Then it upgrades the stack to the latest (unpublished) version of Windmill built for this commit. It expects a docker image with the tag `dev` to be present.
4. It re-runs the tests with the following env var: `WMILL_RUNNING_DEV` set to `1`

Some tests behaves differently depending on `WMILL_RUNNING_DEV`. This way we can test that scripts/flows/schedules... deployed on a previous version of Windmill won't break with the upgrade
Some tests are skipped unless `WMILL_RUNNING_DEV == 1`, those are just regular integration tests that just needs to be run on the dev version.

#### Running locally

Running the tests locally is not easy given that we need to have a Docker image running the latest version of the code. However, the tests simply reaches Windmill API on `http://localhost:8000`, so it can easily be modified to run alongside a simple `cargo run --features enterprise`.

Note that Windmill Enterprise version is required, you'll need a license key for all the tests to run. It needs to be set to the environment variable: `WM_LICENSE_KEY_CI`

For example, to run `identity_script_test.py`, you can do the following:

In one terminal, run your local version of Windmill:

```bash
cargo run --features enterprise
```

Then in another tab, run the test:

```bash
python -m unittest -v test.TestIdentityScript
```

Some tests requires additional setup. This is the case of the SDK tests, which requires Windmill SDK package to be published to either private NPM registry or PyPI server. Custom setup logic can be found in (build.sh)[./build.sh].

#### TODOs

Add integration tests for:

- [ ] Test Python SDK
- [ ] Test job failures as well
- [ ] Test dedicated workers
- [ ] Error handlers and Recovery handlers for schedule
- [ ] Concurrency limits
