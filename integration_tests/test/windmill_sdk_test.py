import datetime
import os
import time
import unittest

from .wmill_integration_test_utils import WindmillClient

VARIABLE_PATH = "u/admin/test_variable"
VARIABLE_VALUE = "Hello world!"

DENO_SCRIPT_PATH = "u/admin/deno_sdk_test_script"
DENO_SCRIPT_VALUE = """
import * as wmill from "npm:windmill-client@{version}"
export async function main() {{
  const val = await wmill.getVariable('u/admin/test_variable')
  return val;
}}
"""

BUN_SCRIPT_PATH = "u/admin/bun_sdk_test_script"
BUN_SCRIPT_VALUE = """
import * as wmill from "windmill-client@{version}"
export async function main() {{
  const val = await wmill.getVariable('u/admin/test_variable')
  return val;
}}
"""

PYTHON_SCRIPT_PATH = "u/admin/python_sdk_test_script"
PYTHON_SCRIPT_VALUE = """
# requirements:
# wmill=={version}
import wmill
def main():
    val = wmill.get_variable("u/admin/test_variable")
    return val
"""

BASH_SCRIPT_PATH = "u/admin/bash_sdk_test_script"
BASH_SCRIPT_VALUE = """
val=$(curl -s -H "Authorization: Bearer $WM_TOKEN" \
  "$BASE_INTERNAL_URL/api/w/$WM_WORKSPACE/variables/get_value/u/admin/test_variable" | jq -r .)
echo "$val"
"""


class TestWindmillSdk(unittest.TestCase):
    _dev_version = os.environ.get("WM_VERSION_DEV", "0.0.0").strip()
    _running_latest = False
    _client: WindmillClient

    @classmethod
    def setUpClass(cls) -> None:
        print("Running {}".format(cls.__name__))
        cls._running_latest = os.environ.get("WMILL_RUNNING_DEV", "0") == "1"
        cls._client = WindmillClient()
        if cls._running_latest:
            cls._client.set_npm_config_registry("http://npm_registry:4873")
            cls._client.create_variable(
                path=VARIABLE_PATH,
                value=VARIABLE_VALUE,
            )
            cls._client.create_script(
                path=DENO_SCRIPT_PATH,
                content=DENO_SCRIPT_VALUE.format(version=cls._dev_version),
                language="deno",
            )
            # TODO: See skipped annotations below
            # cls._client.create_script(
            #     path=BUN_SCRIPT_PATH,
            #     content=BUN_SCRIPT_VALUE.format(version=cls._dev_version),
            #     language="bun",
            # )
            # cls._client.create_script(
            #     path=PYTHON_SCRIPT_PATH,
            #     content=PYTHON_SCRIPT_VALUE.format(version=cls._dev_version),
            #     language="python3",
            # )
            cls._client.create_script(
                path=BASH_SCRIPT_PATH,
                content=BASH_SCRIPT_VALUE,
                language="bash",
            )

    @classmethod
    def tearDownClass(cls) -> None:
        if cls._running_latest:
            cls._client.set_npm_config_registry("")
            cls._client.delete_script(path=DENO_SCRIPT_PATH)
            # TODO: See skipped annotations below
            # cls._client.delete_script(path=BUN_SCRIPT_PATH)
            # cls._client.delete_script(path=PYTHON_SCRIPT_PATH)
            cls._client.delete_script(path=BASH_SCRIPT_PATH)
            cls._client.delete_variable(path=VARIABLE_PATH)
            cls._client.set_npm_config_registry("")

    @unittest.skipUnless(
        os.environ.get("WMILL_RUNNING_DEV", "0") == "1", "Runs on dev version only"
    )
    def test_deno_sdk_usable(self):
        result = self._client.run_sync(DENO_SCRIPT_PATH, {})
        self.assertEqual(result, VARIABLE_VALUE)

    @unittest.skipUnless(
        # os.environ.get("WMILL_RUNNING_DEV", "0") == "1", "Runs on dev version only"
        False,
        "TODO: Skipped for now b/c verdaccio doesn't support trailing slash at the end of URL",
    )
    def test_bun_sdk_usable(self):
        result = self._client.run_sync(BUN_SCRIPT_PATH, {})
        self.assertEqual(result, VARIABLE_VALUE)

    @unittest.skipUnless(
        # os.environ.get("WMILL_RUNNING_DEV", "0") == "1", "Runs on dev version only"
        False,
        "TODO: Need to publish python SDK to private PiPY server",
    )
    def test_python_sdk_usable(self):
        result = self._client.run_sync(PYTHON_SCRIPT_PATH, {})
        self.assertEqual(result, VARIABLE_VALUE)

    @unittest.skipUnless(
        os.environ.get("WMILL_RUNNING_DEV", "0") == "1", "Runs on dev version only"
    )
    def test_bash_sdk_usable(self):
        result = self._client.run_sync(BASH_SCRIPT_PATH, {})
        self.assertEqual(result, VARIABLE_VALUE)
