# wmill

The client for the [Windmill](https://windmill.dev) platform

## Quickstart

```python
import wmill

# with a WM_TOKEN env variable
client = wmill.Client()

# without a WM_TOKEN env variable
client = wmill.Client(token="<mytoken>")

def main():

    version = client.get_version()
    resource = client.get_resource("u/user/resource_path")

    # run synchronously, will return the result
    res = client.run_script_sync(hash="000000000000002a", args={})
    print(res)

    for _ in range(3):
        # run asynchrnously, will return immediately. Can be scheduled
        client.run_script_async(hash="000000000000002a", args={}, scheduled_in_secs=10)
```
