import unittest
import base64
import json
import os
import time
import atexit
import docker
from docker.errors import NotFound

from .wmill_integration_test_utils import WindmillClient

AGENT_CONTAINER_NAME = "windmill_agent_test"

# Global cleanup function registered with atexit
def cleanup_docker_container():
    print(f"At exit: Attempting to remove container {AGENT_CONTAINER_NAME}")
    try:
        docker_client = docker.from_env()
        try:
            container = docker_client.containers.get(AGENT_CONTAINER_NAME)
            container.stop()
            container.remove()
            print(f"At exit: Successfully removed container {AGENT_CONTAINER_NAME}")
        except NotFound:
            print(f"At exit: Container {AGENT_CONTAINER_NAME} not found")
        except Exception as e:
            print(f"At exit: Error removing container: {e}")
    except Exception as e:
        print(f"At exit: Could not initialize Docker client: {e}")

atexit.register(cleanup_docker_container)


class TestAgentWorkers(unittest.TestCase):
    _client: WindmillClient
    _docker_client = None
    _agent_container = None
    _container_name = AGENT_CONTAINER_NAME
    _agent_token = None

    @classmethod
    def setUpClass(cls) -> None:
        print("Running {}".format(cls.__name__))
        cls._client = WindmillClient()

        cls._client.add_global_custom_tag("agent_test")
        cls._docker_client = docker.from_env()
        cls._start_agent_container()
        cls._wait_for_agent_connection()

    @classmethod
    def _start_agent_container(cls):
        wm_image = os.environ.get("WM_IMAGE", "ghcr.io/windmill-labs/windmill-ee")
        wm_version = os.environ.get("WM_VERSION", "latest")

        cls._agent_token = cls._client.create_agent_token(
            worker_group="agent",
            tags=["agent", "python3", "bash", "agent_test"],
            exp=int(time.time()) + 3600  # 1 hour from now
        )

        try:
            container = cls._docker_client.containers.get(cls._container_name)
            container.stop()
            container.remove()
            print(f"Removed existing container {cls._container_name}")
        except NotFound:
            pass

        # Get the host IP address for connecting back to the Windmill server
        host_ip = "host.docker.internal"
        if os.name == "posix" and os.uname().sysname == "Linux":
            import socket
            s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            s.connect(("8.8.8.8", 80))
            host_ip = s.getsockname()[0]
            s.close()

        print(f"Starting agent container connecting to Windmill server at http://{host_ip}:8000")

        # Create and start the container
        container = cls._docker_client.containers.run(
            f"{wm_image}:{wm_version}",
            name=cls._container_name,
            detach=True,
            environment={
                "BASE_INTERNAL_URL": f"http://{host_ip}:8000",
                "MODE": "agent",
                "AGENT_TOKEN": cls._agent_token
            },
            volumes={
                "/var/run/docker.sock": {"bind": "/var/run/docker.sock", "mode": "rw"}
            },
            restart_policy={"Name": "unless-stopped"},
            mem_limit="2g",
            cpu_count=1,
        )

        cls._agent_container = container
        print(f"Started agent container: {cls._container_name}")

    @classmethod
    def _wait_for_agent_connection(cls):
        print("Waiting for agent to connect to the server...")
        connected = False
        max_attempts = 60
        for attempt in range(max_attempts):
            # Query server for connected workers
            workers = cls._client.get_workers_list(ping_since=60)

            # Check if any worker is in the "agent" worker_group
            for worker in workers:
                if worker.get("worker_group") == "agent":
                    connected = True
                    print(f"Agent connected with details: {worker}")
                    break

            if connected:
                break

            print(f"Waiting for agent to connect... Attempt {attempt+1}/{max_attempts}")
            time.sleep(1)

        if not connected:
            raise Exception("Agent failed to connect within the expected time")

        print("Agent successfully connected!")

    @classmethod
    def tearDownClass(cls) -> None:
        print("Cleaning up after tests...")

        if hasattr(cls, "_client") and cls._client is not None:
            cls._client.remove_global_custom_tag("agent_test")

        if cls._docker_client is None:
            try:
                cls._docker_client = docker.from_env()
            except Exception as e:
                print(f"Error initializing Docker client for cleanup: {e}")
                return

        if cls._agent_container is not None:
            try:
                cls._agent_container.stop()
                cls._agent_container.remove()
                print(f"Cleaned up container using container object: {cls._container_name}")
                return
            except Exception as e:
                print(f"Error cleaning up container using object: {e}")

        try:
            container = cls._docker_client.containers.get(cls._container_name)
            container.stop()
            container.remove()
            print(f"Cleaned up container by name: {cls._container_name}")
        except NotFound:
            print(f"Container {cls._container_name} not found during cleanup")
        except Exception as e:
            print(f"Error cleaning up container by name: {e}")

        print("Cleanup complete")

    def test_create_agent_token(self):
        token = self._agent_token
        print(f"Agent token tests for token: {token}")
        self.assertIsNotNone(token)

        # JWT tokens have the format: jwt_agent_<HEADER.PAYLOAD.SIGNATURE>
        prefix = "jwt_agent_"
        self.assertTrue(token.startswith(prefix), "Token should start with jwt_agent_")

        # Extract the JWT by stripping the known prefix (don't split on '_'
        # because base64url encoding uses '_' as a valid character)
        jwt_part = token[len(prefix):]
        self.assertGreater(len(jwt_part), 0, "JWT part should not be empty")
        self.assertEqual(jwt_part.count('.'), 2, "JWT should contain exactly 2 dots")

        # Check that the token contains three base64-encoded parts
        jwt_segments = jwt_part.split('.')
        self.assertEqual(len(jwt_segments), 3, "JWT should have 3 segments")
        for segment in jwt_segments:
            self.assertGreater(len(segment), 0, "JWT segment should not be empty")

        # Decode the JWT payload (second segment)
        payload_base64 = jwt_segments[1]

        # Add padding if needed
        padding_needed = len(payload_base64) % 4
        if padding_needed:
            payload_base64 += '=' * (4 - padding_needed)

        # Base64 decode the payload
        payload_bytes = base64.urlsafe_b64decode(payload_base64)
        payload_json = payload_bytes.decode('utf-8')
        payload = json.loads(payload_json)

        # Check payload structure
        self.assertIn('worker_group', payload, "Payload should contain worker_group")
        self.assertEqual(payload['worker_group'], 'agent', "worker_group should be 'agent'")

        self.assertIn('suffix', payload, "Payload should contain suffix")
        self.assertIsNone(payload['suffix'], "suffix should be null")

        self.assertIn('tags', payload, "Payload should contain tags")
        self.assertIsInstance(payload['tags'], list, "tags should be a list")
        self.assertIn('agent', payload['tags'], "tags should contain 'agent'")

        self.assertIn('exp', payload, "Payload should contain exp")
        self.assertIsInstance(payload['exp'], int, "exp should be an integer")

    def test_agent_is_connected(self):
        """Test that the agent is connected to the server."""
        workers = self._client.get_workers_list(ping_since=60)

        # Find the agent worker
        agent_worker = None
        for worker in workers:
            if worker.get("worker_group") == "agent":
                agent_worker = worker
                break

        self.assertIsNotNone(agent_worker, "Agent worker should be connected")
        self.assertEqual(agent_worker.get("worker_group"), "agent")

        # Check tags
        tags = agent_worker.get("custom_tags", [])
        self.assertIn("agent", tags)
        self.assertIn("python3", tags)
        self.assertIn("bash", tags)
        self.assertIn("agent_test", tags)

    def test_bash_script_on_agent(self):
        """Test running a bash script on the agent worker."""
        # Create a simple bash script tagged to run on the agent
        script_path = "u/admin/agent_bash_test"
        script_content = """
#!/bin/bash
msg="$1"
echo "Running on $(hostname)"
echo "Argument received: $msg"
echo $msg
"""

        # Create the script with the agent_test tag so it runs on our agent
        self._client.create_script(
            path=script_path,
            content=script_content,
            language="bash",
            tag="agent_test"
        )

        try:
            # Run the script
            result = self._client.run_sync(script_path, {"msg": "Hello from agent test!"})

            print(f"Script result: {result}")

            # Verify the result
            self.assertIsNotNone(result)
            self.assertEqual(result, "Hello from agent test!")

        finally:
            # Clean up the script
            self._client.delete_script(script_path)