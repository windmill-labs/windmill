import datetime
import time
import httpx
import json
import os
import uuid



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
            timeout=60.0,  # Go/Rust compilation can take 10+ seconds on first run
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

    def update_script(self, path: str, content: str, language: str, tag: str = None):
        """Update an existing script by creating a new version with parent_hash."""
        print(f"Updating script {path}")

        # Get current script hash
        response = self._client.get(
            f"/api/w/{self._workspace}/scripts/get/p/{path}"
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())
        current_hash = response.json()["hash"]

        payload = {
            "path": path,
            "content": content,
            "description": "",
            "summary": "",
            "language": language,
            "parent_hash": current_hash,
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

    def create_resource(self, path: str, resource_type: str, value: dict, update_if_exists: bool = False):
        print(f"Creating resource {path} of type {resource_type}")
        params = {}
        if update_if_exists:
            params["update_if_exists"] = "true"
        response = self._client.post(
            f"/api/w/{self._workspace}/resources/create",
            json={
                "path": path,
                "value": value,
                "resource_type": resource_type,
                "description": "",
            },
            params=params,
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())
        return response.content.decode()

    def configure_git_sync(self, git_sync_settings: dict):
        print(f"Configuring git sync with {len(git_sync_settings.get('repositories', []))} repositories")
        response = self._client.post(
            f"/api/w/{self._workspace}/workspaces/edit_git_sync_config",
            json={"git_sync_settings": git_sync_settings},
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())
        return response.content.decode()

    def get_completed_jobs(self, job_kinds: str = None, success: bool = None):
        params = {"per_page": 1000}
        if job_kinds:
            params["job_kinds"] = job_kinds
        if success is not None:
            params["success"] = str(success).lower()
        response = self._client.get(
            f"/api/w/{self._workspace}/jobs/completed/list",
            params=params,
        )
        if response.status_code // 100 != 2:
            raise Exception(response.content.decode())
        return response.json()

    def wait_for_sync_jobs(self, initial_count: int, min_new: int = 1, timeout: int = 90) -> list:
        """Poll completed DeploymentCallback jobs until count increases by min_new."""
        start = time.time()
        while time.time() - start < timeout:
            jobs = self.get_completed_jobs(job_kinds="deploymentcallback")
            current_count = len(jobs)
            if current_count >= initial_count + min_new:
                return jobs
            time.sleep(2)
        raise TimeoutError(
            f"Timed out waiting for sync jobs: expected {initial_count + min_new}, "
            f"got {current_count} after {timeout}s"
        )

    def count_deployment_callback_jobs(self) -> int:
        jobs = self.get_completed_jobs(job_kinds="deploymentcallback")
        return len(jobs)

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


GITEA_HOST_URL = os.environ.get("GITEA_HOST_URL", "http://localhost:3000")
GITEA_DOCKER_URL = os.environ.get("GITEA_DOCKER_URL", "http://gitea:3000")
GITEA_ADMIN_USER = "windmill"
GITEA_ADMIN_PASSWORD = "password123!"
GITEA_ADMIN_EMAIL = "windmill@windmill.dev"


class GiteaClient:
    _host_url: str
    _docker_url: str
    _token: str

    def __init__(self):
        self._host_url = GITEA_HOST_URL
        self._docker_url = GITEA_DOCKER_URL
        self._token = None

    def setup_admin(self):
        """Create the admin user in Gitea (idempotent) and get an API token."""
        with httpx.Client(base_url=self._host_url, timeout=30.0) as client:
            # Create admin user (ignore 422 if exists)
            resp = client.post(
                "/api/v1/admin/users",
                json={
                    "username": GITEA_ADMIN_USER,
                    "password": GITEA_ADMIN_PASSWORD,
                    "email": GITEA_ADMIN_EMAIL,
                    "must_change_password": False,
                    "visibility": "public",
                },
                headers={"Content-Type": "application/json"},
                auth=(GITEA_ADMIN_USER, GITEA_ADMIN_PASSWORD),
            )
            if resp.status_code == 201:
                print(f"Created Gitea admin user '{GITEA_ADMIN_USER}'")
            elif resp.status_code == 422:
                print(f"Gitea admin user '{GITEA_ADMIN_USER}' already exists")
            elif resp.status_code == 401:
                # Admin user doesn't exist yet; use the Gitea setup API
                resp2 = client.post(
                    "/user/sign_up",
                    data={
                        "user_name": GITEA_ADMIN_USER,
                        "password": GITEA_ADMIN_PASSWORD,
                        "retype": GITEA_ADMIN_PASSWORD,
                        "email": GITEA_ADMIN_EMAIL,
                    },
                )
                if resp2.status_code // 100 != 2 and resp2.status_code != 303:
                    # Try the API endpoint for creating the first user
                    resp3 = client.post(
                        "/api/v1/admin/users",
                        json={
                            "username": GITEA_ADMIN_USER,
                            "password": GITEA_ADMIN_PASSWORD,
                            "email": GITEA_ADMIN_EMAIL,
                            "must_change_password": False,
                        },
                    )
                    if resp3.status_code // 100 != 2:
                        raise Exception(f"Failed to create Gitea user: {resp3.status_code} {resp3.text}")
                print(f"Created Gitea admin user '{GITEA_ADMIN_USER}' via signup")
            else:
                raise Exception(f"Failed to create Gitea user: {resp.status_code} {resp.text}")

            # Create API token
            token_name = f"integration-test-{uuid.uuid4().hex[:8]}"
            resp = client.post(
                f"/api/v1/users/{GITEA_ADMIN_USER}/tokens",
                json={"name": token_name, "scopes": ["all"]},
                auth=(GITEA_ADMIN_USER, GITEA_ADMIN_PASSWORD),
            )
            if resp.status_code // 100 != 2:
                raise Exception(f"Failed to create Gitea token: {resp.status_code} {resp.text}")
            self._token = resp.json()["sha1"]
            print(f"Created Gitea API token: {token_name}")

    def _headers(self):
        return {
            "Authorization": f"token {self._token}",
            "Content-Type": "application/json",
        }

    def create_repo(self, name: str) -> str:
        """Create a repo and return the docker-internal clone URL with credentials."""
        with httpx.Client(base_url=self._host_url, timeout=30.0) as client:
            resp = client.post(
                "/api/v1/user/repos",
                json={
                    "name": name,
                    "auto_init": True,
                    "default_branch": "main",
                    "private": False,
                },
                headers=self._headers(),
            )
            if resp.status_code // 100 != 2:
                raise Exception(f"Failed to create repo {name}: {resp.status_code} {resp.text}")
            print(f"Created Gitea repo: {name}")
        return f"{self._docker_url}/{GITEA_ADMIN_USER}/{name}.git"

    def get_host_clone_url(self, name: str) -> str:
        """Return host-accessible clone URL with credentials."""
        return f"http://{GITEA_ADMIN_USER}:{GITEA_ADMIN_PASSWORD}@localhost:3000/{GITEA_ADMIN_USER}/{name}.git"

    def get_docker_clone_url(self, name: str) -> str:
        """Return docker-internal clone URL with credentials."""
        return f"http://{GITEA_ADMIN_USER}:{GITEA_ADMIN_PASSWORD}@gitea:3000/{GITEA_ADMIN_USER}/{name}.git"

    def delete_repo(self, name: str):
        with httpx.Client(base_url=self._host_url, timeout=30.0) as client:
            resp = client.delete(
                f"/api/v1/repos/{GITEA_ADMIN_USER}/{name}",
                headers=self._headers(),
            )
            if resp.status_code // 100 == 2 or resp.status_code == 404:
                print(f"Deleted Gitea repo: {name}")
            else:
                print(f"Warning: failed to delete repo {name}: {resp.status_code}")
