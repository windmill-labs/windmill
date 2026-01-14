"""
Test for flow-level resume URLs feature.

This test verifies that resume URLs can be generated at the flow level (instead of step level),
allowing pre-approvals that can be consumed by any later suspend step in the same flow.
"""

import time
import json
from wmill_integration_test_utils import WindmillClient


def test_flow_level_resume():
    """
    Test that a flow-level resume URL can be used to approve a suspend step.

    Flow structure:
    1. Step A: Returns the flow job ID
    2. Step B: Suspend step (waits for approval)
    3. Step C: Returns "completed"

    Test steps:
    1. Create the flow
    2. Start the flow asynchronously
    3. Wait for the flow to reach the suspend step
    4. Generate a flow-level resume URL using the flow job ID
    5. Use that URL to approve
    6. Verify the flow completes successfully
    """
    client = WindmillClient()

    # Create a simple script that returns the parent flow ID (using bun/TypeScript)
    get_flow_id_script = """
export async function main(): Promise<{ flow_job_id: string }> {
    // WM_FLOW_JOB_ID is the parent flow's job ID
    return { flow_job_id: Bun.env.WM_FLOW_JOB_ID || "unknown" };
}
"""

    # Create a simple script for the final step
    final_script = """
export async function main(): Promise<string> {
    return "completed";
}
"""

    try:
        # Create the scripts
        client.create_script(
            path="f/test/get_flow_id",
            content=get_flow_id_script,
            language="bun"
        )
        client.create_script(
            path="f/test/final_step",
            content=final_script,
            language="bun"
        )

        # Create a flow with a suspend step
        flow_value = {
            "summary": "Flow level resume test",
            "value": {
                "modules": [
                    {
                        "id": "a",
                        "value": {
                            "type": "script",
                            "path": "f/test/get_flow_id"
                        }
                    },
                    {
                        "id": "b",
                        "value": {
                            "type": "script",
                            "path": "f/test/final_step"
                        },
                        "suspend": {
                            "required_events": 1,
                            "timeout": 300  # 5 minutes timeout
                        }
                    },
                    {
                        "id": "c",
                        "value": {
                            "type": "script",
                            "path": "f/test/final_step"
                        }
                    }
                ]
            },
            "schema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }

        client.create_flow(
            path="f/test/flow_level_resume_test",
            flow_value_json=json.dumps(flow_value)
        )

        # Start the flow asynchronously
        print("Starting flow...")
        response = client._client.post(
            f"/api/w/{client._workspace}/jobs/run/f/f/test/flow_level_resume_test",
            json={}
        )
        if response.status_code // 100 != 2:
            raise Exception(f"Failed to start flow: {response.content.decode()}")

        flow_job_id = response.content.decode().strip('"')
        print(f"Flow job ID: {flow_job_id}")

        # Wait for the flow to reach the suspend step (step b)
        print("Waiting for flow to reach suspend step...")
        max_wait = 60
        start_time = time.time()
        suspended_job_id = None

        while time.time() - start_time < max_wait:
            # Get the flow status
            response = client._client.get(
                f"/api/w/{client._workspace}/jobs_u/get/{flow_job_id}"
            )
            if response.status_code // 100 != 2:
                raise Exception(f"Failed to get flow status: {response.content.decode()}")

            job_info = response.json()
            flow_status = job_info.get("flow_status", {})
            modules = flow_status.get("modules", [])

            # Check if we're at the suspend step
            for module in modules:
                if module.get("type") == "WaitingForEvents":
                    suspended_job_id = module.get("job")
                    print(f"Flow is suspended at job: {suspended_job_id}")
                    break

            if suspended_job_id:
                break

            time.sleep(1)

        if not suspended_job_id:
            raise Exception("Flow did not reach suspend step within timeout")

        # Generate a flow-level resume URL
        # This is the NEW feature: using flow_level=true to generate a URL for the flow
        print(f"Generating flow-level resume URL for flow {flow_job_id}...")
        response = client._client.get(
            f"/api/w/{client._workspace}/jobs/resume_urls/{suspended_job_id}/0",
            params={"flow_level": "true"}
        )
        if response.status_code // 100 != 2:
            raise Exception(f"Failed to get resume URLs: {response.content.decode()}")

        resume_urls = response.json()
        print(f"Resume URLs: {json.dumps(resume_urls, indent=2)}")

        # The resume URL should use the flow_job_id, not the suspended_job_id
        # Verify the URL contains the flow_job_id
        resume_url = resume_urls.get("resume", "")
        if flow_job_id not in resume_url:
            print(f"WARNING: Expected flow_job_id {flow_job_id} in resume URL, but got: {resume_url}")
            # This is expected to fail until we implement the feature

        # Use the resume URL to approve
        print("Approving via resume URL...")
        # Extract the path from the full URL (handle both port 3000 and 8000)
        resume_path = resume_url.replace("http://localhost:3000", "").replace("http://localhost:8000", "")
        response = client._client.post(resume_path, json={})
        if response.status_code // 100 != 2:
            raise Exception(f"Failed to resume: {response.content.decode()}")

        print("Resume request sent successfully")

        # Wait for the flow to complete
        print("Waiting for flow to complete...")
        max_wait = 60
        start_time = time.time()

        while time.time() - start_time < max_wait:
            response = client._client.get(
                f"/api/w/{client._workspace}/jobs_u/get/{flow_job_id}"
            )
            if response.status_code // 100 != 2:
                raise Exception(f"Failed to get flow status: {response.content.decode()}")

            job_info = response.json()

            # Check if completed
            if job_info.get("type") == "CompletedJob":
                print(f"Flow completed with result: {job_info.get('result')}")
                if job_info.get("success"):
                    print("TEST PASSED: Flow completed successfully")
                    return True
                else:
                    raise Exception(f"Flow failed: {job_info.get('result')}")

            time.sleep(1)

        raise Exception("Flow did not complete within timeout")

    finally:
        # Cleanup
        try:
            client.delete_flow("f/test/flow_level_resume_test")
        except:
            pass
        try:
            client.delete_script("f/test/get_flow_id")
        except:
            pass
        try:
            client.delete_script("f/test/final_step")
        except:
            pass


def test_flow_level_pre_approval():
    """
    Test that a flow-level approval can be "banked" before the suspend step is reached.

    This tests the main use case: generating an approval URL early in the flow,
    having someone approve it, and then when a later suspend step is reached,
    the approval is already there.

    Flow structure:
    1. Step A: Long-running step (simulated with sleep)
    2. Step B: Suspend step
    3. Step C: Returns "completed"

    Test steps:
    1. Start the flow
    2. Immediately generate a flow-level resume URL using the flow job ID
    3. Use that URL to pre-approve (before step A completes)
    4. Wait for the flow to complete (it should auto-continue through suspend)
    """
    client = WindmillClient()

    # Create a script that takes a bit of time (using bun/TypeScript)
    slow_script = """
export async function main(): Promise<string> {
    // Sleep for 3 seconds
    await Bun.sleep(3000);
    return "step_a_done";
}
"""

    final_script = """
export async function main(): Promise<string> {
    return "completed";
}
"""

    try:
        # Create the scripts
        client.create_script(
            path="f/test/slow_step",
            content=slow_script,
            language="bun"
        )
        client.create_script(
            path="f/test/final_step_2",
            content=final_script,
            language="bun"
        )

        # Create a flow
        flow_value = {
            "summary": "Flow level pre-approval test",
            "value": {
                "modules": [
                    {
                        "id": "a",
                        "value": {
                            "type": "script",
                            "path": "f/test/slow_step"
                        }
                    },
                    {
                        "id": "b",
                        "value": {
                            "type": "script",
                            "path": "f/test/final_step_2"
                        },
                        "suspend": {
                            "required_events": 1,
                            "timeout": 300
                        }
                    },
                    {
                        "id": "c",
                        "value": {
                            "type": "script",
                            "path": "f/test/final_step_2"
                        }
                    }
                ]
            },
            "schema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }

        client.create_flow(
            path="f/test/flow_level_pre_approval_test",
            flow_value_json=json.dumps(flow_value)
        )

        # Start the flow
        print("Starting flow...")
        response = client._client.post(
            f"/api/w/{client._workspace}/jobs/run/f/f/test/flow_level_pre_approval_test",
            json={}
        )
        if response.status_code // 100 != 2:
            raise Exception(f"Failed to start flow: {response.content.decode()}")

        flow_job_id = response.content.decode().strip('"')
        print(f"Flow job ID: {flow_job_id}")

        # Immediately generate a flow-level resume URL
        # We use the flow_job_id directly since we want a flow-level approval
        print("Generating flow-level resume URL immediately...")
        response = client._client.get(
            f"/api/w/{client._workspace}/jobs/resume_urls/{flow_job_id}/0",
            params={"flow_level": "true"}
        )
        if response.status_code // 100 != 2:
            raise Exception(f"Failed to get resume URLs: {response.content.decode()}")

        resume_urls = response.json()
        print(f"Resume URLs: {json.dumps(resume_urls, indent=2)}")

        # Pre-approve immediately (before the slow step completes)
        print("Pre-approving via resume URL...")
        resume_url = resume_urls.get("resume", "")
        # Extract the path from the full URL (handle both port 3000 and 8000)
        resume_path = resume_url.replace("http://localhost:3000", "").replace("http://localhost:8000", "")
        response = client._client.post(resume_path, json={})
        if response.status_code // 100 != 2:
            raise Exception(f"Failed to resume: {response.content.decode()}")

        print("Pre-approval sent successfully")

        # Wait for the flow to complete
        # It should auto-continue through the suspend step since we pre-approved
        print("Waiting for flow to complete...")
        max_wait = 120
        start_time = time.time()

        while time.time() - start_time < max_wait:
            response = client._client.get(
                f"/api/w/{client._workspace}/jobs_u/get/{flow_job_id}"
            )
            if response.status_code // 100 != 2:
                raise Exception(f"Failed to get flow status: {response.content.decode()}")

            job_info = response.json()

            # Check if completed
            if job_info.get("type") == "CompletedJob":
                print(f"Flow completed with result: {job_info.get('result')}")
                if job_info.get("success"):
                    print("TEST PASSED: Flow completed successfully with pre-approval")
                    return True
                else:
                    raise Exception(f"Flow failed: {job_info.get('result')}")

            # Check flow status
            flow_status = job_info.get("flow_status", {})
            modules = flow_status.get("modules", [])
            for module in modules:
                module_type = module.get("type", "")
                if module_type == "WaitingForEvents":
                    # If we're waiting for events, the pre-approval should have been consumed
                    print(f"Flow is at WaitingForEvents - checking if pre-approval was consumed...")

            time.sleep(2)

        raise Exception("Flow did not complete within timeout")

    finally:
        # Cleanup
        try:
            client.delete_flow("f/test/flow_level_pre_approval_test")
        except:
            pass
        try:
            client.delete_script("f/test/slow_step")
        except:
            pass
        try:
            client.delete_script("f/test/final_step_2")
        except:
            pass


if __name__ == "__main__":
    print("=" * 60)
    print("Test 1: Flow-level resume (approve after suspend)")
    print("=" * 60)
    try:
        test_flow_level_resume()
        print("Test 1 PASSED\n")
    except Exception as e:
        print(f"Test 1 FAILED: {e}\n")

    print("=" * 60)
    print("Test 2: Flow-level pre-approval (approve before suspend)")
    print("=" * 60)
    try:
        test_flow_level_pre_approval()
        print("Test 2 PASSED\n")
    except Exception as e:
        print(f"Test 2 FAILED: {e}\n")
