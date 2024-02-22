import json
import os
import time
import unittest

from .wmill_integration_test_utils import WindmillClient

PATH = "u/admin/crashing_flow"
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
            },
            "crash_on": 
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
            "x",
            "crash_on"
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
                    "lock": "",
                    "type": "rawscript",
                    "content": "def main(x: int):\\n    return x",
                    "language": "python3",
                    "input_transforms":
                    {
                        "x":
                        {
                            "expr": "flow_input.x",
                            "type": "javascript"
                        }
                    }
                }
            },
            {
                "id": "b",
                "value":
                {
                    "lock": "",
                    "type": "rawscript",
                    "content": "def main(x: int):\\n    return x",
                    "language": "python3",
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
                    "content": "def main(x: int):\\n    return x",
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
            }
        ]
    },
    "extra_perms":
    {},
    "ws_error_handler_muted": false
}
"""


class TestCrashingFlow(unittest.TestCase):
    _client: WindmillClient

    @classmethod
    def setUpClass(cls) -> None:
        print("Running {}".format(cls.__name__))
        cls._client = WindmillClient()
        if os.environ.get("WMILL_RUNNING_DEV", False):
            cls._client.create_flow(
                path=PATH,
                flow_value_json=FLOW_VALUE,
            )

    @classmethod
    def tearDownClass(cls) -> None:
        if os.environ.get("WMILL_RUNNING_DEV", False):
            cls._client.delete_flow(path=PATH)

    @unittest.skipUnless(
        os.environ.get("WMILL_RUNNING_DEV", "0") == "1", "Runs on dev version only"
    )
    def test_flow_cancelled(self):
        job_id = self._client.run_async(PATH, {"x": 5, "crash_on": "b"}, type="f")
        time.sleep(
            6
        )  # in test mode, monitor runs every 3 seconds, waiting 6 should be enough
        job_status = self._client.get_job_status(job_id)
        self.assertEqual(job_status["success"], False)
        self.assertEqual(job_status["canceled"], True)
        self.assertEqual(job_status["canceled_by"], "monitor")
        self.assertEqual(
            job_status["canceled_reason"],
            "Flow cancelled as it was hanging in between 2 steps",
        )

    @unittest.skipUnless(
        os.environ.get("WMILL_RUNNING_DEV", "0") == "1", "Runs on dev version only"
    )
    def test_flow__restarted(self):
        job_id = self._client.run_async(PATH, {"x": 5, "crash_on": "a"}, type="f")
        time.sleep(
            6
        )  # in test mode, monitor runs every 3 seconds, waiting 6 should be enough
        job_status = self._client.get_job_status(job_id)
        self.assertEqual(job_status["canceled"], False)
        self._client.force_cancel(job_id)
