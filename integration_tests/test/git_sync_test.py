import tempfile
import time
import unittest
import uuid

import git as gitpython

from .wmill_integration_test_utils import WindmillClient, GiteaClient


# Hub script used by Windmill's git sync feature
GIT_SYNC_SCRIPT_PATH = "hub/28102/sync-script-to-git-repo-windmill"


def unique_name(prefix: str = "git-sync-test") -> str:
    return f"{prefix}-{uuid.uuid4().hex[:8]}"


class TestGitSync(unittest.TestCase):
    _client: WindmillClient
    _gitea: GiteaClient
    _repos_to_cleanup: list

    @classmethod
    def setUpClass(cls) -> None:
        print("Running {}".format(cls.__name__))
        cls._client = WindmillClient()
        cls._gitea = GiteaClient()
        cls._gitea.setup_admin()
        cls._repos_to_cleanup = []

    @classmethod
    def tearDownClass(cls) -> None:
        # Disable git sync to avoid interfering with other tests
        try:
            cls._client.configure_git_sync({"repositories": []})
        except Exception as e:
            print(f"Warning: failed to disable git sync: {e}")

        for repo_name in cls._repos_to_cleanup:
            cls._gitea.delete_repo(repo_name)

    def _create_test_repo(self) -> tuple:
        """Create a Gitea repo and return (repo_name, docker_clone_url)."""
        name = unique_name()
        docker_url = self._gitea.create_repo(name)
        self._repos_to_cleanup.append(name)
        return name, docker_url

    def _setup_git_sync_resource(self, repo_name: str, branch: str = "main") -> str:
        """Create a git_repository resource pointing to the Gitea repo.
        Returns the resource path."""
        resource_path = f"u/admin/git_sync_{repo_name.replace('-', '_')}"
        docker_url = self._gitea.get_docker_clone_url(repo_name)
        self._client.create_resource(
            path=resource_path,
            resource_type="git_repository",
            value={
                "url": docker_url,
                "branch": branch,
                "is_github_app": False,
            },
            update_if_exists=True,
        )
        return resource_path

    def _configure_single_repo_sync(
        self,
        resource_path: str,
        include_type=None,
        include_path=None,
        use_individual_branch=False,
        group_by_folder=False,
        force_branch=None,
    ):
        """Configure git sync with a single repository."""
        repo_settings = {
            "script_path": GIT_SYNC_SCRIPT_PATH,
            "git_repo_resource_path": f"$res:{resource_path}",
            "use_individual_branch": use_individual_branch,
            "group_by_folder": group_by_folder,
        }
        if force_branch:
            repo_settings["force_branch"] = force_branch
        if include_type or include_path:
            repo_settings["settings"] = {
                "include_type": include_type or [],
                "include_path": include_path if include_path is not None else ["**"],
            }

        self._client.configure_git_sync({
            "repositories": [repo_settings],
        })

    def _clone_repo(self, repo_name: str) -> str:
        """Clone the repo to a temp dir and return the path."""
        host_url = self._gitea.get_host_clone_url(repo_name)
        tmp_dir = tempfile.mkdtemp()
        gitpython.Repo.clone_from(host_url, tmp_dir)
        return tmp_dir

    def _list_repo_files(self, repo_dir: str) -> list:
        """List all tracked files in the repo (relative paths)."""
        repo = gitpython.Repo(repo_dir)
        return [item.path for item in repo.head.commit.tree.traverse()]

    def _get_commit_count(self, repo_dir: str, branch: str = "main") -> int:
        repo = gitpython.Repo(repo_dir)
        return len(list(repo.iter_commits(branch)))

    def _get_branches(self, repo_dir: str) -> list:
        repo = gitpython.Repo(repo_dir)
        return [ref.name for ref in repo.remote().refs]

    def test_script_deploy_syncs_to_git(self):
        """Deploy a script and verify it appears in the git repo."""
        repo_name, _ = self._create_test_repo()
        resource_path = self._setup_git_sync_resource(repo_name)
        self._configure_single_repo_sync(
            resource_path,
            include_type=["script"],
        )

        initial_count = self._client.count_deployment_callback_jobs()

        script_path = f"u/admin/{unique_name('sync_test')}"
        self._client.create_script(
            path=script_path,
            content="def main():\n    return 42\n",
            language="python3",
        )

        self._client.wait_for_sync_jobs(initial_count, min_new=1)

        # Give a moment for git push to complete
        time.sleep(3)

        repo_dir = self._clone_repo(repo_name)
        files = self._list_repo_files(repo_dir)

        # The script should appear somewhere in the repo
        matching = [f for f in files if script_path in f]
        self.assertTrue(
            len(matching) > 0,
            f"Expected script '{script_path}' in repo files: {files}",
        )

    def test_multi_repo_routing(self):
        """Two repos with different path filters receive the correct objects."""
        repo_name_a, _ = self._create_test_repo()
        repo_name_b, _ = self._create_test_repo()
        res_path_a = self._setup_git_sync_resource(repo_name_a)
        res_path_b = self._setup_git_sync_resource(repo_name_b)

        folder_a = unique_name("folder_a")
        folder_b = unique_name("folder_b")

        self._client.configure_git_sync({
            "repositories": [
                {
                    "script_path": GIT_SYNC_SCRIPT_PATH,
                    "git_repo_resource_path": f"$res:{res_path_a}",
                    "use_individual_branch": False,
                    "group_by_folder": False,
                    "settings": {
                        "include_type": ["script"],
                        "include_path": [f"f/{folder_a}/**"],
                    },
                },
                {
                    "script_path": GIT_SYNC_SCRIPT_PATH,
                    "git_repo_resource_path": f"$res:{res_path_b}",
                    "use_individual_branch": False,
                    "group_by_folder": False,
                    "settings": {
                        "include_type": ["script"],
                        "include_path": [f"f/{folder_b}/**"],
                    },
                },
            ],
        })

        # Create the folders first
        try:
            self._client._client.post(
                f"/api/w/{self._client._workspace}/folders/create",
                json={"name": folder_a},
            )
        except Exception:
            pass
        try:
            self._client._client.post(
                f"/api/w/{self._client._workspace}/folders/create",
                json={"name": folder_b},
            )
        except Exception:
            pass

        initial_count = self._client.count_deployment_callback_jobs()

        script_a = f"f/{folder_a}/script_a"
        script_b = f"f/{folder_b}/script_b"

        self._client.create_script(
            path=script_a,
            content="def main():\n    return 'a'\n",
            language="python3",
        )
        self._client.create_script(
            path=script_b,
            content="def main():\n    return 'b'\n",
            language="python3",
        )

        # Wait for at least 2 deployment callback jobs
        self._client.wait_for_sync_jobs(initial_count, min_new=2)
        time.sleep(3)

        # Verify repo A has script_a but not script_b
        repo_dir_a = self._clone_repo(repo_name_a)
        files_a = self._list_repo_files(repo_dir_a)
        self.assertTrue(
            any("script_a" in f for f in files_a),
            f"Expected script_a in repo A files: {files_a}",
        )
        self.assertFalse(
            any("script_b" in f for f in files_a),
            f"Did not expect script_b in repo A files: {files_a}",
        )

        # Verify repo B has script_b but not script_a
        repo_dir_b = self._clone_repo(repo_name_b)
        files_b = self._list_repo_files(repo_dir_b)
        self.assertTrue(
            any("script_b" in f for f in files_b),
            f"Expected script_b in repo B files: {files_b}",
        )
        self.assertFalse(
            any("script_a" in f for f in files_b),
            f"Did not expect script_a in repo B files: {files_b}",
        )

    def test_script_update_creates_new_commit(self):
        """Updating a script should produce a new commit in the git repo."""
        repo_name, _ = self._create_test_repo()
        resource_path = self._setup_git_sync_resource(repo_name)
        self._configure_single_repo_sync(resource_path, include_type=["script"])

        script_path = f"u/admin/{unique_name('update_test')}"

        # Create initial script
        initial_count = self._client.count_deployment_callback_jobs()
        self._client.create_script(
            path=script_path,
            content="def main():\n    return 1\n",
            language="python3",
        )
        self._client.wait_for_sync_jobs(initial_count, min_new=1)
        time.sleep(3)

        repo_dir = self._clone_repo(repo_name)
        initial_commits = self._get_commit_count(repo_dir)

        # Update the script (update_script creates new version with parent_hash)
        update_count = self._client.count_deployment_callback_jobs()
        self._client.update_script(
            path=script_path,
            content="def main():\n    return 2\n",
            language="python3",
        )
        self._client.wait_for_sync_jobs(update_count, min_new=1)
        time.sleep(3)

        # Re-clone and check commit count increased
        repo_dir2 = self._clone_repo(repo_name)
        new_commits = self._get_commit_count(repo_dir2)
        self.assertGreater(
            new_commits,
            initial_commits,
            f"Expected more commits after update: {new_commits} vs {initial_commits}",
        )

    def test_force_branch(self):
        """Verify that force_branch config is accepted and sync still works.
        force_branch is an env-level option (controls which branch the repo
        resource points to), not a separate push target."""
        repo_name, _ = self._create_test_repo()
        resource_path = self._setup_git_sync_resource(repo_name)
        self._configure_single_repo_sync(
            resource_path,
            include_type=["script"],
            force_branch="main",
        )

        initial_count = self._client.count_deployment_callback_jobs()
        script_path = f"u/admin/{unique_name('branch_test')}"
        self._client.create_script(
            path=script_path,
            content="def main():\n    return 'branch'\n",
            language="python3",
        )
        self._client.wait_for_sync_jobs(initial_count, min_new=1)
        time.sleep(3)

        repo_dir = self._clone_repo(repo_name)
        files = self._list_repo_files(repo_dir)
        matching = [f for f in files if script_path in f]
        self.assertTrue(
            len(matching) > 0,
            f"Expected script in repo with force_branch: {files}",
        )

    def test_individual_branch_mode(self):
        """Verify that individual_branch mode creates separate branches."""
        repo_name, _ = self._create_test_repo()
        resource_path = self._setup_git_sync_resource(repo_name)
        self._configure_single_repo_sync(
            resource_path,
            include_type=["script"],
            use_individual_branch=True,
        )

        initial_count = self._client.count_deployment_callback_jobs()
        script_path = f"u/admin/{unique_name('ind_branch')}"
        self._client.create_script(
            path=script_path,
            content="def main():\n    return 'individual'\n",
            language="python3",
        )
        self._client.wait_for_sync_jobs(initial_count, min_new=1)
        time.sleep(3)

        repo_dir = self._clone_repo(repo_name)
        branches = self._get_branches(repo_dir)

        # In individual branch mode, a branch named after the script path should exist
        # (with slashes replaced by some separator)
        non_main_branches = [b for b in branches if "main" not in b and "HEAD" not in b]
        self.assertTrue(
            len(non_main_branches) > 0,
            f"Expected at least one non-main branch in individual mode, got: {branches}",
        )

    def test_deploy_multiple_object_types(self):
        """Deploy a script, flow, and variable and verify all appear in the repo."""
        repo_name, _ = self._create_test_repo()
        resource_path = self._setup_git_sync_resource(repo_name)
        self._configure_single_repo_sync(
            resource_path,
            include_type=["script", "flow", "variable"],
        )

        initial_count = self._client.count_deployment_callback_jobs()

        suffix = unique_name("multi")
        script_path = f"u/admin/{suffix}_script"
        flow_path = f"u/admin/{suffix}_flow"
        var_path = f"u/admin/{suffix}_var"

        self._client.create_script(
            path=script_path,
            content="def main():\n    return 'multi'\n",
            language="python3",
        )
        self._client.create_flow(
            path=flow_path,
            flow_value_json="""{
                "summary": "test flow",
                "value": {
                    "modules": [{
                        "id": "a",
                        "value": {
                            "type": "rawscript",
                            "content": "def main():\\n    return 1",
                            "language": "python3",
                            "input_transforms": {},
                            "tag": ""
                        }
                    }]
                },
                "schema": {
                    "$schema": "https://json-schema.org/draft/2020-12/schema",
                    "properties": {},
                    "required": [],
                    "type": "object",
                    "order": []
                }
            }""",
        )
        self._client.create_variable(
            path=var_path,
            value="test_value",
        )

        # Wait for 3 deployment callbacks (one per object)
        self._client.wait_for_sync_jobs(initial_count, min_new=3)
        time.sleep(3)

        repo_dir = self._clone_repo(repo_name)
        files = self._list_repo_files(repo_dir)
        files_str = "\n".join(files)

        self.assertTrue(
            any(suffix + "_script" in f for f in files),
            f"Expected script in repo:\n{files_str}",
        )
        self.assertTrue(
            any(suffix + "_flow" in f for f in files),
            f"Expected flow in repo:\n{files_str}",
        )
        self.assertTrue(
            any(suffix + "_var" in f for f in files),
            f"Expected variable in repo:\n{files_str}",
        )

    def test_group_by_folder(self):
        """Verify that group_by_folder organizes files by folder structure."""
        repo_name, _ = self._create_test_repo()
        resource_path = self._setup_git_sync_resource(repo_name)
        self._configure_single_repo_sync(
            resource_path,
            include_type=["script"],
            group_by_folder=True,
        )

        initial_count = self._client.count_deployment_callback_jobs()
        script_path = f"u/admin/{unique_name('folder_test')}"
        self._client.create_script(
            path=script_path,
            content="def main():\n    return 'folder'\n",
            language="python3",
        )
        self._client.wait_for_sync_jobs(initial_count, min_new=1)
        time.sleep(3)

        repo_dir = self._clone_repo(repo_name)
        files = self._list_repo_files(repo_dir)

        # With group_by_folder, files should be organized with folder prefixes
        # (e.g., u/admin/script_name.py instead of flat)
        matching = [f for f in files if script_path.split("/")[-1] in f]
        self.assertTrue(
            len(matching) > 0,
            f"Expected script in repo with group_by_folder: {files}",
        )
