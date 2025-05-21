import datetime
import time
import httpx
import json
import os



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
                "name": self._workspace
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

    def create_script(self, path: str, content: str, language: str, tag: str = None):
        print(f"Creating script {path}")

        payload = {
            "path": path,
            "content": content,
            "description": "",
            "summary": "",
            "language": language,
        }

        if tag is not None:
            payload["tag"] = tag

        response = self._client.post(
            f"/api/w/{self._workspace}/scripts/create",
            json=payload,
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

    def get_global_custom_tags(self):
        """
        Get the current list of global custom tags.

        Returns:
            list: List of custom tags or empty list if not set or an error occurred.
        """
        try:
            response = self._client.get("/api/settings/global/custom_tags")
            if response.status_code // 100 == 2:
                tags = response.json()
                return tags if tags is not None else []
            else:
                print(f"Error retrieving global custom tags: Status {response.status_code}, Response: {response.content.decode()}")
                return []
        except Exception as e:
            print(f"Exception when retrieving global custom tags: {e}")
            return []

    def add_global_custom_tag(self, tag):
        """
        Add a tag to the global custom tags if it's not already present.

        Args:
            tag (str): The tag to add to global custom tags.

        Returns:
            bool: True if the tag was added or already exists, False if there was an error.
        """
        try:
            current_tags = self.get_global_custom_tags()

            if tag in current_tags:
                print(f"Tag '{tag}' already exists in global custom tags")
                return True

            new_tags = current_tags + [tag]
            print(f"Adding '{tag}' to global custom tags: {new_tags}")

            response = self._client.post(
                "/api/settings/global/custom_tags",
                json={
                    "value": new_tags,
                },
            )

            if response.status_code // 100 == 2:
                print(f"Successfully added '{tag}' to global custom tags")
                return True
            else:
                print(f"Error adding tag to global custom tags: Status {response.status_code}, Response: {response.content.decode()}")
                return False
        except Exception as e:
            print(f"Exception when adding global custom tag: {e}")
            return False

    def remove_global_custom_tag(self, tag):
        """
        Remove a tag from the global custom tags if it exists.

        Args:
            tag (str): The tag to remove from global custom tags.

        Returns:
            bool: True if the tag was removed or didn't exist, False if there was an error.
        """
        try:
            current_tags = self.get_global_custom_tags()

            if tag not in current_tags:
                print(f"Tag '{tag}' doesn't exist in global custom tags")
                return True

            new_tags = [t for t in current_tags if t != tag]
            print(f"Removing '{tag}' from global custom tags: {new_tags}")

            response = self._client.post(
                "/api/settings/global/custom_tags",
                json={
                    "value": new_tags,
                },
            )

            if response.status_code // 100 == 2:
                print(f"Successfully removed '{tag}' from global custom tags")
                return True
            else:
                print(f"Error removing tag from global custom tags: Status {response.status_code}, Response: {response.content.decode()}")
                return False
        except Exception as e:
            print(f"Exception when removing global custom tag: {e}")
            return False

    def get_workers_list(self, ping_since=60, page=0, per_page=100):
        """
        Get a list of workers currently connected to the Windmill server.

        Args:
            ping_since (int): Only include workers that have pinged in the last N seconds. Default is 60.
            page (int): Page number for pagination. Default is 0.
            per_page (int): Number of results per page. Default is 100.

        Returns:
            list: List of worker objects or empty list if no workers found or an error occurred.
        """
        try:
            params = {
                "page": page,
                "per_page": per_page,
                "ping_since": ping_since
            }

            response = self._client.get(
                "/api/workers/list",
                params=params
            )
            if response.status_code // 100 == 2:
                return response.json()
            else:
                print(f"Error retrieving workers list: Status {response.status_code}, Response: {response.content.decode()}")
                return []
        except Exception as e:
            print(f"Exception when retrieving workers list: {e}")
            return []

    def create_agent_token(self, worker_group="agent", tags=None, exp=None):
        """
        Create an agent JWT token using superadmin privilege.

        Args:
            worker_group (str): The worker group for the agent, defaults to "agent"
            tags (list): Tags for the agent, defaults to ["agent"]
            exp (int): Expiration timestamp, defaults to a timestamp about 1 year in the future

        Returns:
            str: The JWT token for the agent
        """
        if tags is None:
            tags = ["agent"]

        if exp is None:
            exp = int(time.time()) + 31536000  # 60*60*24*365 = 1 year

        print(f"Creating agent token for worker_group={worker_group}, tags={tags}")
        response = self._client.post(
            "/api/agent_workers/create_agent_token",
            json={
                "worker_group": worker_group,
                "tags": tags,
                "exp": exp
            },
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())

        token = response.content.decode().strip('"')
        print(f"Created agent token: {token}")
        return token
