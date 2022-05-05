# wmill

The core client for the [Windmill](https://windmill.dev) platform.

It is a convenient wrapper around the exhaustive, automatically generated from
OpenApi but less user-friendly
[windmill-api](https://pypi.org/project/windmill-api/).

## Quickstart

```python
import wmill


def main():
    #os.environ.set("WM_TOKEN", "<mytoken>") OPTIONAL to set token used by the wmill client
    version = wmill.get_version()
    resource = wmill.get_resource("u/user/resource_path")

    # run synchronously, will return the result
    res = wmill.run_script_sync(hash="000000000000002a", args={})
    print(res)

    for _ in range(3):
        # run asynchrnously, will return immediately. Can be scheduled
        wmill.run_script_async(hash="000000000000002a", args={}, scheduled_in_secs=10)
```
