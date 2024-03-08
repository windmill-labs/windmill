import datetime
import os
import time
import unittest

from .wmill_integration_test_utils import WindmillClient

FLOW_SCHEDULE_PATH = "u/admin/flow_schedule"
FLOW_PATH = "u/admin/scheduled_flow"
FLOW_VALUE = """
{
    "summary": "",
    "value":
    {
        "modules":
        [
            {
                "id": "a",
                "value":
                {
                    "type": "rawscript",
                    "content": "def main(x: int):\\n    return x",
                    "language": "python3",
                    "input_transforms":
                    {
                        "x":
                        {
                            "type": "javascript",
                            "expr": "flow_input.x"
                        }
                    },
                    "tag": ""
                }
            }
        ]
    },
    "edited_by": "",
    "edited_at": "",
    "archived": false,
    "extra_perms":
    {},
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
    }
}
"""

SCRIPT_SCHEDULE_PATH = "u/admin/script_schedule"
SCRIPT_PATH = "u/admin/scheduled_script"
SCRIPT_VALUE = """
def main(x: int):
    return x
"""


class TestSchedule(unittest.TestCase):
    _client: WindmillClient

    @classmethod
    def setUpClass(cls) -> None:
        print("Running {}".format(cls.__name__))
        cls._client = WindmillClient()
        if not os.environ.get("WMILL_RUNNING_DEV", False):
            cls._client.create_flow(
                path=FLOW_PATH,
                flow_value_json=FLOW_VALUE,
            )
            cls._client.create_script(
                path=SCRIPT_PATH,
                content=SCRIPT_VALUE,
                language="python3",
            )
            cls._client.create_schedule(
                path=FLOW_SCHEDULE_PATH,
                runnable_path=FLOW_PATH,
                type="flow",
                args={"x": 5},
            )
            cls._client.create_schedule(
                path=SCRIPT_SCHEDULE_PATH,
                runnable_path=SCRIPT_PATH,
                args={"x": 8},
            )

    @classmethod
    def tearDownClass(cls) -> None:
        if os.environ.get("WMILL_RUNNING_DEV", False):
            cls._client.delete_schedule(path=FLOW_SCHEDULE_PATH)
            cls._client.delete_schedule(path=SCRIPT_SCHEDULE_PATH)
            cls._client.delete_flow(path=FLOW_PATH)
            cls._client.delete_script(path=SCRIPT_PATH)

    @staticmethod
    def parse_db_datetime(db_datetime: str) -> datetime.datetime:
        return datetime.datetime.fromisoformat(db_datetime.strip("Z") + "+00:00")

    def test_script_schedule_running(self):
        # the script is scheduled to run every 5 seconds, wait for 6 before checking is has run
        time.sleep(6)
        script_runs = self._client.get_latest_job_runs(path=SCRIPT_PATH)
        if len(script_runs) == 0:
            self.fail("No script runs found")
        latest_run = script_runs[0]
        latest_run_time = TestSchedule.parse_db_datetime(latest_run["created_at"])
        time_now = datetime.datetime.now(datetime.timezone.utc)
        # check that last run is within 5 seconds of now
        self.assertTrue(time_now - latest_run_time < datetime.timedelta(seconds=5))

    def test_flow_schedule_running(self):
        # the flow is scheduled to run every 5 seconds, wait for 6 before checking is has run
        time.sleep(6)
        flow_runs = self._client.get_latest_job_runs(path=FLOW_PATH)
        if len(flow_runs) == 0:
            self.fail("No flow runs found")
        latest_run = flow_runs[0]
        latest_run_time = TestSchedule.parse_db_datetime(latest_run["created_at"])
        time_now = datetime.datetime.now(datetime.timezone.utc)
        # check that last run is within 5 seconds of now
        self.assertTrue(time_now - latest_run_time < datetime.timedelta(seconds=5))
