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
        elif l == 3:
            script_path = "/".join(splitted)
            import requests

            url = f"{os.environ.get('BASE_INTERNAL_URL')}/api/w/{os.environ.get('WM_WORKSPACE')}/scripts/raw/p/{script_path}"

            r = requests.get(
                url, headers={"Authorization": f"Bearer {os.environ.get('WM_TOKEN')}"}
            )

            if r.status_code == 200:
                folder = os.getcwd() + "/tmp/" + "/".join(splitted[:-1])
                fullpath = folder + "/" + splitted[-1] + ".py"
                os.makedirs(folder, exist_ok=True)
                with open(fullpath, "w+") as f:
                    f.write(r.text)
                return ModuleSpec(name, SourceFileLoader(name, fullpath))
            else:
                print(r.text, r.status_code)
                raise ImportError(f"Script {script_path} not found")
        else:
            raise ImportError(
                "Import can only be done at the top level of a folder or user space"
            )


sys.meta_path.append(WindmillFinder)
