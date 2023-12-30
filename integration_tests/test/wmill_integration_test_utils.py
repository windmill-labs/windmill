import datetime
import time
import httpx
import json
import os

USERNAME = "admin"


class WindmillClient:
    _url: str
    _token: str
    _workspace: str

    _client: httpx.Client

    def __init__(self):
        self._workspace = "integration-tests"
        self._url = "http://localhost:8000"
        self._token = self._login()

        self._client = self._init_client()
        print("New client for Windmill version {}".format(self.get_version()))
        self._create_workspace()
        self._set_license_key()

    def __del__(self):
        self._logout()
        self._client.close()

    def _login(self) -> str:
        with httpx.Client(base_url=self._url) as unauth_client:
            response = unauth_client.post(
                "/api/auth/login",
                json={
                    "email": "admin@windmill.dev",
                    "password": "changeme",
                },
            )
            if response.status_code // 100 != 2:
                raise Exception(response.content.decode())
            return response.content.decode()

    def _logout(self) -> None:
        response = self._client.post(
            "/api/auth/logout",
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())

    def _init_client(self):
        token = self._token
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {token}",
        }
        return httpx.Client(
            base_url=self._url,
            headers=headers,
        )

    def _set_license_key(self):
        license_key = os.environ.get("LICENSE_KEY", "").strip()
        print(
            "Setting license key to {}...{}".format(license_key[:15], license_key[-15:])
        )
        response = self._client.post(
            "/api/settings/global/license_key",
            json={
                "value": license_key,
            },
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())

    def _create_workspace(self):
        print("Creating workspace for integration tests")
        exists = self._client.post(
            "/api/workspaces/exists",
            json={
                "id": self._workspace,
            },
        )
        if exists.status_code // 100 == 2 and exists.content.decode() == "true":
            print("Workspace already exists, not creating it")
            return
        response = self._client.post(
            "/api/workspaces/create",
            json={
                "id": self._workspace,
                "name": self._workspace,
                "username": "admin",
            },
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())
        return response.content.decode()

    def set_npm_config_registry(self, registry_url: str):
        response = self._client.post(
            "/api/settings/global/npm_config_registry",
            json={
                "value": registry_url,
            },
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())

    def run_sync(self, path: str, args: dict, type: str = "p"):
        print(f"Running {path} with args {args}")
        response = self._client.post(
            f"/api/w/{self._workspace}/jobs/run_wait_result/{type}/{path}",
            json=args,
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())
        return response.json()

    def create_script(self, path: str, content: str, language: str):
        print(f"Creating script {path}")
        response = self._client.post(
            f"/api/w/{self._workspace}/scripts/create",
            json={
                "path": path,
                "content": content,
                "description": "",
                "summary": "",
                "language": language,
            },
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())
        script_hash = response.content.decode()
        print(f"Script hash for path {path} is {script_hash}")
        time_now = datetime.datetime.now(datetime.timezone.utc)
        while datetime.datetime.now(
            datetime.timezone.utc
        ) - time_now < datetime.timedelta(seconds=60):
            response = self._client.get(
                f"/api/w/{self._workspace}/scripts/deployment_status/h/{script_hash}"
            )
            if response.status_code // 100 != 2:
                raise Exception(response.content.decode())
            elif response.json()["lock"] != None:
                # deployment successful -> return
                return
            elif response.json()["lock_error_logs"] != None:
                raise Exception(response.json()["lock_error_logs"])
            print(f"Waiting for script {path} with hash {script_hash} to be deployed")
            time.sleep(1)
        raise Exception(f"Script deployment failed for {path}")

    def delete_script(self, path: str):
        print(f"Deleting script {path}")
        response = self._client.post(
            f"/api/w/{self._workspace}/scripts/delete/p/{path}",
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())
        return response.content.decode()

    def create_flow(self, path: str, flow_value_json: str):
        print(f"Creating flow {path}")
        parsed_flow = json.loads(flow_value_json)
        if "path" not in parsed_flow:
            parsed_flow["path"] = path
        response = self._client.post(
            f"/api/w/{self._workspace}/flows/create",
            json=parsed_flow,
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())
        return response.content.decode()

    def delete_flow(self, path: str):
        print(f"Deleting flow {path}")
        response = self._client.delete(
            f"/api/w/{self._workspace}/flows/delete/{path}",
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())
        return response.content.decode()

    def create_schedule(
        self,
        path: str,
        runnable_path: str,
        type: str = "script",
        schedule: str = "*/5 * * * * *",
        args: dict = {},
    ):
        print(f"Creating schedule {path}")
        response = self._client.post(
            f"/api/w/{self._workspace}/schedules/create",
            json={
                "path": path,
                "schedule": schedule,
                "timezone": "Europe/Paris",
                "script_path": runnable_path,
                "is_flow": type == "flow",
                "args": args,
                "enabled": True,
            },
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())
        return response.content.decode()

    def delete_schedule(self, path: str):
        print(f"Deleting schedule {path}")
        response = self._client.delete(
            f"/api/w/{self._workspace}/schedules/delete/{path}",
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())
        return response.content.decode()

    def create_variable(
        self,
        path: str,
        value: str,
    ):
        print(f"Creating variable {path} with value '{value}'")
        response = self._client.post(
            f"/api/w/{self._workspace}/variables/create",
            json={
                "path": path,
                "value": value,
                "description": "",
                "is_secret": False,
            },
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())
        return response.content.decode()

    def delete_variable(self, path: str):
        print(f"Deleting variable {path}")
        response = self._client.delete(
            f"/api/w/{self._workspace}/variables/delete/{path}",
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())
        return response.content.decode()

    def get_latest_job_runs(self, path: str):
        response = self._client.get(
            f"/api/w/{self._workspace}/jobs/list?script_path_exact={path}"
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())
        return response.json()

    def get_version(self):
        response = self._client.get("/api/version")
        return response.content.decode()
