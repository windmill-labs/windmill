import sys
import os
from importlib.abc import MetaPathFinder, Loader
from importlib.machinery import ModuleSpec, SourceFileLoader
import urllib.response


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
            folder = os.getcwd() + "/tmp/" + "/".join(splitted[:-1])
            fullpath = folder + "/" + splitted[-1] + ".py"

            if os.path.exists(fullpath):
                return ModuleSpec(name, SourceFileLoader(name, fullpath))


            import urllib.parse
            import urllib.request

            headers = {
                "Authorization": f"Bearer {os.environ.get('WM_TOKEN')}",
                "User-Agent": "windmill/beta"
            }

            query_params = "?cache_folders=true"
            runnable_id = os.environ.get('WM_RUNNABLE_ID')
            if runnable_id:
                query_params += f"&cache_key={runnable_id}"
            url = f"{os.environ.get('BASE_INTERNAL_URL')}/api/w/{os.environ.get('WM_WORKSPACE')}/scripts/raw/p/{script_path}.py{query_params}"

            req = urllib.request.Request(url, None, headers)
            try:
                with urllib.request.urlopen(req) as response:
                    os.makedirs(folder, exist_ok=True)
                    r = response.read().decode("utf-8")
                    if r == "WINDMILL_IS_FOLDER":
                        return ModuleSpec(name, WindmillLoader(name))
                    with open(fullpath, "w+") as f:
                        f.write(r)
                    return ModuleSpec(name, SourceFileLoader(name, fullpath))
            except urllib.error.HTTPError as e:
                if e.code != 404:
                    print(f"Error fetching script {script_path}: HTTP {e.code} - {e.reason}")
                return ModuleSpec(name, WindmillLoader(name))
            except Exception as e:
                print(f"Error fetching script {script_path}: {e}")
                return ModuleSpec(name, WindmillLoader(name))



sys.meta_path.append(WindmillFinder)
