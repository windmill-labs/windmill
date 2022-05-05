# wmill

The postgres extension client for the [Windmill](https://windmill.dev) platform.

[windmill-api](https://pypi.org/project/windmill-api/).

## Quickstart

```python
import wmill_pg


def main():
    my_list = query("UPDATE demo SET value = 'value' RETURNING key, value")
    for key, value in my_list:
        ...
```
