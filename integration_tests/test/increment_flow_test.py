import json
import os
import time
import unittest

from .wmill_integration_test_utils import WindmillClient

PATH = "u/admin/increment_flow"
FLOW_VALUE = """
{
    "summary": "",
    "schema":
    {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "properties":
        {
            "x":
            {
                "type": "integer",
                "description": "",
                "format": ""
            }
        },
        "required":
        [],
        "type": "object",
        "order":
        [
            "x"
        ]
    },
    "value":
    {
        "modules":
        [
            {
                "id": "a",
                "value":
                {
                    "tag": "",
                    "lock": "{\\n  \\"version\\": \\"3\\",\\n  \\"remote\\": {}\\n}\\n",
                    "type": "rawscript",
                    "content": "export async function main(x: number) {\\n  return x + 1\\n}\\n",
                    "language": "deno",
                    "input_transforms":
                    {
                        "x":
                        {
                            "expr": "flow_input.x + 5",
                            "type": "javascript"
                        }
                    }
                }
            },
            {
                "id": "b",
                "value":
                {
                    "lock": "{\\n  \\"dependencies\\": {}\\n}\\n//bun.lockb\\n<empty>",
                    "type": "rawscript",
                    "content": "export async function main(x: number) {\\n  return x + 1\\n}\\n",
                    "language": "bun",
                    "input_transforms":
                    {
                        "x":
                        {
                            "expr": "results.a",
                            "type": "javascript"
                        }
                    }
                }
            },
            {
                "id": "c",
                "value":
                {
                    "lock": "",
                    "type": "rawscript",
                    "content": "def main(x: int):\\n    return x + 1",
                    "language": "python3",
                    "input_transforms":
                    {
                        "x":
                        {
                            "expr": "results.b",
                            "type": "javascript"
                        }
                    }
                }
            },
            {
                "id": "d",
                "value":
                {
                    "lock": "module mymod\\n\\ngo 1.21.5\\n//go.sum\\n",
                    "type": "rawscript",
                    "content": "package inner\\nfunc main(x int) (interface{}, error) {\\n\\treturn x + 1, nil\\n}\\n",
                    "language": "go",
                    "input_transforms":
                    {
                        "x":
                        {
                            "expr": "results.c",
                            "type": "javascript"
                        }
                    }
                }
            },
            {
                "id": "e",
                "value":
                {
                    "lock": "",
                    "type": "rawscript",
                    "content": "x=\\"$1\\"\\necho $((x+1))",
                    "language": "bash",
                    "input_transforms":
                    {
                        "x":
                        {
                            "expr": "`${results.d}`",
                            "type": "javascript"
                        }
                    }
                }
            }
        ]
    },
    "extra_perms":
    {},
    "ws_error_handler_muted": false
}
"""


class TestIncrementFlow(unittest.TestCase):
    _client: WindmillClient

    @classmethod
    def setUpClass(cls) -> None:
        print("Running {}".format(cls.__name__))
        cls._client = WindmillClient()
        if not os.environ.get("WMILL_RUNNING_DEV", False):
            cls._client.create_flow(
                path=PATH,
                flow_value_json=FLOW_VALUE,
            )

    @classmethod
    def tearDownClass(cls) -> None:
        if os.environ.get("WMILL_RUNNING_DEV", False):
            cls._client.delete_flow(path=PATH)

    def test_flow(self):
        result = self._client.run_sync(PATH, {"x": 5}, type="f")
        self.assertEqual(result, "15")  # bash only knows strings
