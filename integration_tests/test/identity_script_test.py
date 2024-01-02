import os
import time
import unittest

from .wmill_integration_test_utils import WindmillClient

PATH_TEMPLATE = "u/admin/{lang}_identity_script"
SCRIPTS = {
    "bash": """
x="$1"
echo ${x}
""",
    "bun": """
export async function main(x: number) {
  return x;
}
""",
    "deno": """
export async function main(x: number) {
  return x;
}
""",
    "go": """
package inner
func main(x int) (interface{}, error) {
	return x, nil
}
""",
    "python3": """
def main(x: int):
    return x
""",
}


class TestIdentityScript(unittest.TestCase):
    _client: WindmillClient

    @classmethod
    def setUpClass(cls) -> None:
        print("Running {}".format(cls.__name__))
        cls._client = WindmillClient()

        if not os.environ.get("WMILL_RUNNING_DEV", False):
            for lang, script in SCRIPTS.items():
                cls._client.create_script(
                    path=PATH_TEMPLATE.format(lang=lang),
                    content=script,
                    language=lang,
                )

    @classmethod
    def tearDownClass(cls) -> None:
        if os.environ.get("WMILL_RUNNING_DEV", False):
            for lang in SCRIPTS:
                cls._client.delete_script(path=PATH_TEMPLATE.format(lang=lang))

    def test_bash(self):
        path = PATH_TEMPLATE.format(lang="bash")
        result = self._client.run_sync(path, {"x": 5})
        self.assertEqual(result, "5")  # bash only knows strings

    def test_bun(self):
        path = PATH_TEMPLATE.format(lang="bun")
        result = self._client.run_sync(path, {"x": 5})
        self.assertEqual(result, 5)

    def test_deno(self):
        path = PATH_TEMPLATE.format(lang="deno")
        result = self._client.run_sync(path, {"x": 5})
        self.assertEqual(result, 5)

    def test_go(self):
        path = PATH_TEMPLATE.format(lang="go")
        result = self._client.run_sync(path, {"x": 5})
        self.assertEqual(result, 5)

    def test_python(self):
        path = PATH_TEMPLATE.format(lang="python3")
        result = self._client.run_sync(path, {"x": 5})
        self.assertEqual(result, 5)
