import sys
import os
from importlib.abc import MetaPathFinder, Loader
from importlib.machinery import ModuleSpec, SourceFileLoader


class WindmillLoader(Loader):
    def __init__(self, path):
        self.path = path

    def create_module(self, spec):
        return None

    def exec_module(self, module):
        module.__path__ = self.path
        return None


class WindmillFinder(MetaPathFinder):
    @classmethod
    def find_spec(cls, name, path, target=None):
        splitted = name.split(".")

        if splitted[0] != "f" and splitted[0] != "u":
            return None
        l = len(splitted)
        if l <= 2:
            return ModuleSpec(name, WindmillLoader(name))
        elif l > 2:
            script_path = "/".join(splitted)
            import urllib.parse
            import urllib.request

            headers = {
                "Authorization": f"Bearer {os.environ.get('WM_TOKEN')}",
                "User-Agent": "windmill/beta"
            }
            url = f"{os.environ.get('BASE_INTERNAL_URL')}/api/w/{os.environ.get('WM_WORKSPACE')}/scripts/raw/p/{script_path}.py"

            req = urllib.request.Request(url, None, headers)
            try:
                with urllib.request.urlopen(req) as response:
                        r = response.read().decode("utf-8")
                        folder = os.getcwd() + "/tmp/" + "/".join(splitted[:-1])
                        fullpath = folder + "/" + splitted[-1] + ".py"
                        os.makedirs(folder, exist_ok=True)
                        with open(fullpath, "w+") as f:
                            f.write(r)
                        return ModuleSpec(name, SourceFileLoader(name, fullpath))
            except:
                # raise ImportError(f"Script {script_path} not found")
                return ModuleSpec(name, WindmillLoader(name))



sys.meta_path.append(WindmillFinder)
