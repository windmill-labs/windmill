Testing pythong wmill client
============================

Make sure you have a local windmill BE running and listening on localhost:8000 (either using cargo run or via the docker compose).

Install the local package to your virtual env:
```bash
# this is necessary only if you made a change to the BE API via the openapi.yml file
cd ./windmill-api # you can generate it using the build.sh script at the root of this repo
pip3 install .

cd ./wmill
pip3 install .
```

Then you can run go to `wmill_client_test.py` and add a Token and Workspace if necessary. You can then implement your own test calling any function in the wmill client and test its output.
