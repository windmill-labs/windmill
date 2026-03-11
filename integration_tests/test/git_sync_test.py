import os
import shutil
import tempfile
import time
import unittest
import uuid

import git as gitpython

from .wmill_integration_test_utils import WindmillClient, GiteaClient


# Script content template for bun/TypeScript scripts
def ts_script(body: str) -> str:
    return f"export async function main() {{\n  {body}\n}}\n"


def unique_name(prefix: str = "git-sync-test") -> str:
    return f"{prefix}-{uuid.uuid4().hex[:8]}"


class TestGitSync(unittest.TestCase):
    _client: WindmillClient
    _gitea: GiteaClient
    _repos_to_cleanup: list
    _fork_workspaces_to_cleanup: list

    @classmethod
    def setUpClass(cls) -> None:
        print("Running {}".format(cls.__name__))
        cls._client = WindmillClient()
        cls._gitea = GiteaClient()
        cls._gitea.setup_admin()
        cls._repos_to_cleanup = []
        cls._fork_workspaces_to_cleanup = []

    @classmethod
    def tearDownClass(cls) -> None:
        # Disable git sync to avoid interfering with other tests
        try:
            cls._client.configure_git_sync({"repositories": []})
        except Exception as e:
            print(f"Warning: failed to disable git sync: {e}")

        for fork_id in cls._fork_workspaces_to_cleanup:
            try:
                cls._client.delete_workspace(fork_id)
            except Exception as e:
                print(f"Warning: failed to delete fork workspace {fork_id}: {e}")

        for repo_name in cls._repos_to_cleanup:
            cls._gitea.delete_repo(repo_name)

    def setUp(self):
        """Wait for any pending deployment callbacks from previous tests to drain."""
        time.sleep(2)
        # Wait until no new deployment callback jobs appear for 4 seconds
        prev_count = self._client.count_deployment_callback_jobs()
        for _ in range(3):
            time.sleep(2)
            cur_count = self._client.count_deployment_callback_jobs()
            if cur_count == prev_count:
                break
            prev_count = cur_count

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
        """Configure git sync with a single repository (auto-managed script)."""
        repo_settings = {
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

    def _clone_repo(self, repo_name: str, branch: str = None) -> str:
        """Clone the repo to a temp dir and return the path."""
        host_url = self._gitea.get_host_clone_url(repo_name)
        tmp_dir = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, tmp_dir, ignore_errors=True)
        args = {}
        if branch:
            args["branch"] = branch
        gitpython.Repo.clone_from(host_url, tmp_dir, **args)
        return tmp_dir

    def _clone_repo_all_branches(self, repo_name: str) -> str:
        """Clone the repo fetching all branches."""
        host_url = self._gitea.get_host_clone_url(repo_name)
        tmp_dir = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, tmp_dir, ignore_errors=True)
        gitpython.Repo.clone_from(host_url, tmp_dir, no_single_branch=True)
        return tmp_dir

    def _list_repo_files(self, repo_dir: str, branch: str = None) -> list:
        """List all tracked files in the repo (relative paths)."""
        repo = gitpython.Repo(repo_dir)
        if branch:
            commit = repo.refs[branch].commit
        else:
            commit = repo.head.commit
        return [item.path for item in commit.tree.traverse()]

    def _read_file_content(self, repo_dir: str, file_path: str) -> str:
        """Read a file's content from the repo working tree."""
        full_path = os.path.join(repo_dir, file_path)
        with open(full_path, "r") as f:
            return f.read()

    def _get_commit_count(self, repo_dir: str, branch: str = "main") -> int:
        repo = gitpython.Repo(repo_dir)
        return len(list(repo.iter_commits(branch)))

    def _get_last_commit_message(self, repo_dir: str, branch: str = "main") -> str:
        repo = gitpython.Repo(repo_dir)
        return repo.iter_commits(branch).__next__().message

    def _get_branches(self, repo_dir: str) -> list:
        repo = gitpython.Repo(repo_dir)
        return [ref.name for ref in repo.remote().refs]

    def _create_folder(self, folder_name: str):
        """Create a folder in the workspace, ignoring errors if it already exists."""
        try:
            self._client._client.post(
                f"/api/w/{self._client._workspace}/folders/create",
                json={"name": folder_name},
            )
        except Exception:
            pass

    # ──────────────────────────────────────────────────
    # Core happy-path tests
    # ──────────────────────────────────────────────────

    def test_script_deploy_syncs_to_git(self):
        """Deploy a script and verify it appears in the git repo with correct content."""
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
            content=ts_script("return 42"),
            language="bun",
        )

        self._client.wait_for_sync_jobs(initial_count, min_new=1)
        time.sleep(3)

        repo_dir = self._clone_repo(repo_name)
        files = self._list_repo_files(repo_dir)

        # The script should appear in the repo
        matching = [f for f in files if script_path in f]
        self.assertTrue(
            len(matching) > 0,
            f"Expected script '{script_path}' in repo files: {files}",
        )

        # Verify file content matches what we deployed
        script_file = [f for f in matching if f.endswith(".ts")][0]
        content = self._read_file_content(repo_dir, script_file)
        self.assertIn(
            "return 42",
            content,
            f"Expected 'return 42' in script content: {content}",
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
                    "git_repo_resource_path": f"$res:{res_path_a}",
                    "use_individual_branch": False,
                    "group_by_folder": False,
                    "settings": {
                        "include_type": ["script"],
                        "include_path": [f"f/{folder_a}/**"],
                    },
                },
                {
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

        self._create_folder(folder_a)
        self._create_folder(folder_b)

        initial_count = self._client.count_deployment_callback_jobs()

        script_a = f"f/{folder_a}/script_a"
        script_b = f"f/{folder_b}/script_b"

        self._client.create_script(
            path=script_a,
            content=ts_script("return 'a'"),
            language="bun",
        )
        self._client.create_script(
            path=script_b,
            content=ts_script("return 'b'"),
            language="bun",
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

    def test_script_update_creates_new_commit_with_updated_content(self):
        """Updating a script should produce a new commit with the new content."""
        repo_name, _ = self._create_test_repo()
        resource_path = self._setup_git_sync_resource(repo_name)
        self._configure_single_repo_sync(resource_path, include_type=["script"])

        script_path = f"u/admin/{unique_name('update_test')}"

        # Create initial script
        initial_count = self._client.count_deployment_callback_jobs()
        self._client.create_script(
            path=script_path,
            content=ts_script("return 1"),
            language="bun",
        )
        self._client.wait_for_sync_jobs(initial_count, min_new=1)
        time.sleep(3)

        repo_dir = self._clone_repo(repo_name)
        initial_commits = self._get_commit_count(repo_dir)

        # Update the script
        update_count = self._client.count_deployment_callback_jobs()
        self._client.update_script(
            path=script_path,
            content=ts_script("return 2"),
            language="bun",
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

        # Verify file content reflects the update
        files = self._list_repo_files(repo_dir2)
        script_file = [f for f in files if script_path in f and f.endswith(".ts")][0]
        content = self._read_file_content(repo_dir2, script_file)
        self.assertIn(
            "return 2",
            content,
            f"Expected 'return 2' in updated script content: {content}",
        )
        self.assertNotIn(
            "return 1",
            content,
            f"Did not expect 'return 1' in updated script content: {content}",
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
            content=ts_script("return 'multi'"),
            language="bun",
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
                            "content": "export async function main() { return 1 }",
                            "language": "bun",
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

    # ──────────────────────────────────────────────────
    # Commit message verification
    # ──────────────────────────────────────────────────

    def test_commit_message_format(self):
        """Verify commit messages have the [WM] prefix."""
        repo_name, _ = self._create_test_repo()
        resource_path = self._setup_git_sync_resource(repo_name)
        self._configure_single_repo_sync(resource_path, include_type=["script"])

        initial_count = self._client.count_deployment_callback_jobs()
        script_path = f"u/admin/{unique_name('commit_msg')}"
        self._client.create_script(
            path=script_path,
            content=ts_script("return 'msg'"),
            language="bun",
        )
        self._client.wait_for_sync_jobs(initial_count, min_new=1)
        time.sleep(3)

        repo_dir = self._clone_repo(repo_name)
        commit_msg = self._get_last_commit_message(repo_dir)

        self.assertTrue(
            commit_msg.startswith("[WM]"),
            f"Expected commit message to start with '[WM]', got: {commit_msg!r}",
        )

    # ──────────────────────────────────────────────────
    # Rename handling
    # ──────────────────────────────────────────────────

    def test_rename_removes_old_file(self):
        """Renaming a script should remove the old file and create the new one."""
        repo_name, _ = self._create_test_repo()
        resource_path = self._setup_git_sync_resource(repo_name)
        self._configure_single_repo_sync(resource_path, include_type=["script"])

        old_path = f"u/admin/{unique_name('rename_old')}"

        # Create initial script
        initial_count = self._client.count_deployment_callback_jobs()
        self._client.create_script(
            path=old_path,
            content=ts_script("return 'old'"),
            language="bun",
        )
        self._client.wait_for_sync_jobs(initial_count, min_new=1)
        time.sleep(3)

        # Verify old script exists in repo
        repo_dir = self._clone_repo(repo_name)
        files = self._list_repo_files(repo_dir)
        old_name = old_path.split("/")[-1]
        self.assertTrue(
            any(old_name in f for f in files),
            f"Expected old script '{old_name}' in repo: {files}",
        )

        # Create new script at different path (simulates rename)
        new_path = f"u/admin/{unique_name('rename_new')}"
        rename_count = self._client.count_deployment_callback_jobs()
        self._client.create_script(
            path=new_path,
            content=ts_script("return 'renamed'"),
            language="bun",
        )
        # Also delete the old script
        self._client.delete_script(old_path)
        # Wait for both create and delete deployment callbacks
        self._client.wait_for_sync_jobs(rename_count, min_new=2)
        time.sleep(3)

        # Verify new script exists
        repo_dir2 = self._clone_repo(repo_name)
        files2 = self._list_repo_files(repo_dir2)
        new_name = new_path.split("/")[-1]
        self.assertTrue(
            any(new_name in f for f in files2),
            f"Expected new script '{new_name}' in repo: {files2}",
        )
        self.assertFalse(
            any(old_name in f for f in files2),
            f"Expected old script '{old_name}' to be removed: {files2}",
        )

    # ──────────────────────────────────────────────────
    # Promotion mode (individual branches)
    # ──────────────────────────────────────────────────

    def test_promotion_mode_creates_per_object_branches(self):
        """In promotion mode (use_individual_branch=True), each deploy creates
        a branch named wm_deploy/{workspace}/{path_type}/{path} with the content
        on that branch, not on main."""
        repo_name, _ = self._create_test_repo()
        resource_path = self._setup_git_sync_resource(repo_name)
        self._configure_single_repo_sync(
            resource_path,
            include_type=["script"],
            use_individual_branch=True,
        )

        initial_count = self._client.count_deployment_callback_jobs()
        script_path = f"u/admin/{unique_name('promo')}"
        self._client.create_script(
            path=script_path,
            content=ts_script("return 'promotion'"),
            language="bun",
        )
        self._client.wait_for_sync_jobs(initial_count, min_new=1)
        time.sleep(3)

        # Clone with all branches
        repo_dir = self._clone_repo_all_branches(repo_name)
        branches = self._get_branches(repo_dir)

        # Should have a branch matching wm_deploy pattern
        wm_branches = [b for b in branches if "wm_deploy/" in b]
        self.assertTrue(
            len(wm_branches) > 0,
            f"Expected wm_deploy/ branch, got branches: {branches}",
        )

        # The branch name should contain 'script' (the path_type)
        deploy_branch = wm_branches[0]
        self.assertIn(
            "script",
            deploy_branch,
            f"Expected 'script' in branch name: {deploy_branch}",
        )

        # The script path (with / replaced by __) should appear in the branch name
        script_name = script_path.split("/")[-1]
        self.assertIn(
            script_name,
            deploy_branch.replace("/", "__"),
            f"Expected script name '{script_name}' in branch: {deploy_branch}",
        )

        # Verify main branch does NOT have the script
        main_files = self._list_repo_files(repo_dir, branch="origin/main")
        self.assertFalse(
            any(script_name in f for f in main_files),
            f"Did not expect script on main branch, but found it: {main_files}",
        )

        # Verify the deploy branch HAS the script
        local_branch_name = deploy_branch.replace("origin/", "")
        repo = gitpython.Repo(repo_dir)
        repo.git.checkout(local_branch_name)
        branch_files = self._list_repo_files(repo_dir)
        self.assertTrue(
            any(script_name in f for f in branch_files),
            f"Expected script on deploy branch '{local_branch_name}': {branch_files}",
        )

    def test_promotion_mode_group_by_folder(self):
        """With use_individual_branch=True and group_by_folder=True, the branch name
        uses the folder prefix (first 2 path segments joined by __) instead of the
        full path."""
        repo_name, _ = self._create_test_repo()
        resource_path = self._setup_git_sync_resource(repo_name)
        self._configure_single_repo_sync(
            resource_path,
            include_type=["script"],
            use_individual_branch=True,
            group_by_folder=True,
        )

        folder_name = unique_name("grp")
        self._create_folder(folder_name)

        initial_count = self._client.count_deployment_callback_jobs()
        script_path = f"f/{folder_name}/{unique_name('grp_script')}"
        self._client.create_script(
            path=script_path,
            content=ts_script("return 'grouped'"),
            language="bun",
        )
        self._client.wait_for_sync_jobs(initial_count, min_new=1)
        time.sleep(3)

        repo_dir = self._clone_repo_all_branches(repo_name)
        branches = self._get_branches(repo_dir)

        wm_branches = [b for b in branches if "wm_deploy/" in b]
        self.assertTrue(
            len(wm_branches) > 0,
            f"Expected wm_deploy/ branch with group_by_folder: {branches}",
        )

        # With group_by_folder, the branch should contain the folder prefix
        # format: wm_deploy/{workspace}/f__{folder_name}
        deploy_branch = wm_branches[0]
        expected_folder_part = f"f__{folder_name}"
        self.assertIn(
            expected_folder_part,
            deploy_branch,
            f"Expected folder-grouped branch name containing '{expected_folder_part}', got: {deploy_branch}",
        )

    # ──────────────────────────────────────────────────
    # force_branch with wmill.yaml
    # ──────────────────────────────────────────────────

    def test_force_branch_with_wmill_yaml(self):
        """force_branch passes --branch to wmill sync pull, which selects the
        matching gitBranches config from wmill.yaml. With branch-specific variables
        configured, the variable file should use a branch-specific path."""
        repo_name, _ = self._create_test_repo()
        resource_path = self._setup_git_sync_resource(repo_name)

        # Push a wmill.yaml to the repo that configures branch-specific variables
        wmill_yaml_content = """\
includes:
  - "**"
gitBranches:
  staging:
    specificItems:
      variables:
        - "**"
"""
        self._gitea.create_file(repo_name, "wmill.yaml", wmill_yaml_content)

        self._configure_single_repo_sync(
            resource_path,
            include_type=["variable"],
            force_branch="staging",
        )

        initial_count = self._client.count_deployment_callback_jobs()
        var_path = f"u/admin/{unique_name('env_var')}"
        self._client.create_variable(
            path=var_path,
            value="staging_value",
        )
        self._client.wait_for_sync_jobs(initial_count, min_new=1)
        time.sleep(3)

        repo_dir = self._clone_repo(repo_name)
        files = self._list_repo_files(repo_dir)

        var_name = var_path.split("/")[-1]

        # With force_branch="staging" and specificItems for variables,
        # the variable file should have ".staging." in its name
        staging_files = [f for f in files if var_name in f and ".staging." in f]
        self.assertTrue(
            len(staging_files) > 0,
            f"Expected variable file with '.staging.' in name for branch-specific item, "
            f"got files: {files}",
        )

    # ──────────────────────────────────────────────────
    # Exclude path filtering
    # ──────────────────────────────────────────────────

    def test_exclude_path_filtering(self):
        """Scripts in excluded paths should not be synced to the repo."""
        repo_name, _ = self._create_test_repo()
        resource_path = self._setup_git_sync_resource(repo_name)

        folder_inc = unique_name("inc")
        folder_exc = unique_name("exc")
        self._create_folder(folder_inc)
        self._create_folder(folder_exc)

        self._client.configure_git_sync({
            "repositories": [{
                "git_repo_resource_path": f"$res:{resource_path}",
                "use_individual_branch": False,
                "group_by_folder": False,
                "settings": {
                    "include_type": ["script"],
                    "include_path": ["f/**"],
                    "exclude_path": [f"f/{folder_exc}/**"],
                },
            }],
        })

        initial_count = self._client.count_deployment_callback_jobs()

        script_inc = f"f/{folder_inc}/included_script"
        script_exc = f"f/{folder_exc}/excluded_script"

        self._client.create_script(
            path=script_inc,
            content=ts_script("return 'included'"),
            language="bun",
        )
        self._client.create_script(
            path=script_exc,
            content=ts_script("return 'excluded'"),
            language="bun",
        )

        # Only 1 sync job expected (the excluded one should not trigger)
        self._client.wait_for_sync_jobs(initial_count, min_new=1)
        time.sleep(5)
        # Verify no extra sync jobs arrived for the excluded script
        final_count = self._client.count_deployment_callback_jobs()
        self.assertEqual(
            final_count, initial_count + 1,
            f"Expected exactly 1 new sync job, got {final_count - initial_count}",
        )

        repo_dir = self._clone_repo(repo_name)
        files = self._list_repo_files(repo_dir)

        self.assertTrue(
            any("included_script" in f for f in files),
            f"Expected included_script in repo: {files}",
        )
        self.assertFalse(
            any("excluded_script" in f for f in files),
            f"Did not expect excluded_script in repo: {files}",
        )

    # ──────────────────────────────────────────────────
    # Workspace fork
    # ──────────────────────────────────────────────────

    def test_workspace_fork_creates_branch(self):
        """Forking a workspace with git sync configured should create a
        fork branch in the git repo."""
        repo_name, _ = self._create_test_repo()
        resource_path = self._setup_git_sync_resource(repo_name)

        # Configure git sync on the parent workspace (sync mode, not promotion)
        self._configure_single_repo_sync(
            resource_path,
            include_type=["script"],
        )

        # Deploy a script first so there's content in the repo
        initial_count = self._client.count_deployment_callback_jobs()
        script_path = f"u/admin/{unique_name('fork_base')}"
        self._client.create_script(
            path=script_path,
            content=ts_script("return 'base'"),
            language="bun",
        )
        self._client.wait_for_sync_jobs(initial_count, min_new=1)
        time.sleep(3)

        # Create workspace fork
        fork_id = f"wm-fork-{uuid.uuid4().hex[:8]}"
        fork_name = f"Fork {fork_id}"
        self._fork_workspaces_to_cleanup.append(fork_id)

        # Step 1: Create git branches for the fork
        job_ids = self._client.create_workspace_fork_branch(fork_id, fork_name)
        if job_ids:
            self._client.wait_for_jobs_by_ids(job_ids, timeout=90)
            time.sleep(3)

        # Step 2: Create the fork workspace
        self._client.create_workspace_fork(fork_id, fork_name)

        # Verify a fork branch was created in the git repo
        repo_dir = self._clone_repo_all_branches(repo_name)
        branches = self._get_branches(repo_dir)

        # Fork branches are named: wm-fork/{original_branch}/{fork_id}
        fork_branches = [b for b in branches if "wm-fork" in b]
        self.assertTrue(
            len(fork_branches) > 0,
            f"Expected a wm-fork branch in the repo after forking, got: {branches}",
        )
