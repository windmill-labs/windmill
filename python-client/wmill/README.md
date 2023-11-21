# wmill

The core client for the [Windmill](https://windmill.dev) platform.


## Quickstart

```python
import wmill


def main():
    
    client = wmill.Windmill(
        # token=...  <- this is optional. otherwise the client will look for the WM_TOKEN env var
    )
    
    print(client.version)
    print(client.get("u/user/resource_path"))
    
    job_id = client.create_job(path="path/to/script")
    print(job_id)
    
    return client.run_script(path="path/to/script", args={"arg1": "value1"})
```
