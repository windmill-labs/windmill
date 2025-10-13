import sys
import os
from importlib.abc import MetaPathFinder, Loader
from importlib.machinery import ModuleSpec, SourceFileLoader
import time

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
        l = len(splitted)  # noqa: E741
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
            
            for attempt in range(4):  # 0, 1, 2, 3 = up to 3 retries
                try:
                    req_start = time.time()
                    with urllib.request.urlopen(req) as response:
                        os.makedirs(folder, exist_ok=True)
                        r = response.read().decode("utf-8")
                        if r == "WINDMILL_IS_FOLDER":
                            return ModuleSpec(name, WindmillLoader(name))
                        with open(fullpath, "w+") as f:
                            f.write(r)
                        return ModuleSpec(name, SourceFileLoader(name, fullpath))
                except urllib.error.HTTPError as e:
                    duration = time.time() - req_start
                    if e.code != 404:
                        print(f"Error fetching script {script_path}: HTTP {e.code} - {e.reason} - {duration}s")
                    return ModuleSpec(name, WindmillLoader(name))
                except Exception as e:
                    duration = time.time() - req_start
                    # Check if this is errno 104 (Connection reset by peer) and we have retries left
                    if (hasattr(e, 'errno') and e.errno == 104) and attempt < 3:
                        print(f"Connection reset (errno 104) fetching script {script_path}, retrying in 3s (attempt {attempt + 1}/3)")
                        time.sleep(3)
                        continue
                    print(f"Error fetching script {script_path}: {e} - {duration}s")
                    return ModuleSpec(name, WindmillLoader(name))



sys.meta_path.append(WindmillFinder)
