#!/usr/bin/env python3
"""
Test script for the DAP WebSocket server.
This script connects to the server and tests breakpoint functionality.

Run the server first:
    python dap_websocket_server.py

Then run this test:
    python test_dap_server.py
"""

import asyncio
import json
import sys

try:
    import websockets
except ImportError:
    print("websockets package required. Install with: pip install websockets")
    sys.exit(1)

# Test Python script with known breakpoints
TEST_SCRIPT = """
x = 1
y = 2
z = x + y
print(f"Result: {z}")
w = z * 2
print(f"Final: {w}")
"""

# Line numbers where we want to set breakpoints (1-indexed)
BREAKPOINT_LINES = [3, 5]  # z = x + y, w = z * 2

# Test script with main() function (Windmill style)
TEST_SCRIPT_WITH_MAIN = """
def main(x: str, count: int = 1):
    print(f"Starting with x={x}, count={count}")
    result = x * count
    print(f"Result: {result}")
    return result
"""

# Breakpoints for the main() test: lines 3 and 4 (inside main function)
MAIN_BREAKPOINT_LINES = [3, 4]


class DAPTestClient:
    def __init__(self, url: str = "ws://localhost:5679"):
        self.url = url
        self.ws = None
        self.seq = 1
        self.pending_requests: dict[int, asyncio.Future] = {}
        self.events: list[dict] = []
        self.stopped_events: list[dict] = []

    async def connect(self):
        print(f"Connecting to {self.url}...")
        self.ws = await websockets.connect(self.url)
        print("Connected!")
        # Start message receiver
        asyncio.create_task(self._receive_messages())

    async def disconnect(self):
        if self.ws:
            await self.ws.close()

    async def _receive_messages(self):
        try:
            async for message in self.ws:
                data = json.loads(message)
                print(f"<-- Received: {json.dumps(data, indent=2)}")

                if data.get("type") == "response":
                    req_seq = data.get("request_seq")
                    if req_seq in self.pending_requests:
                        self.pending_requests[req_seq].set_result(data)
                elif data.get("type") == "event":
                    self.events.append(data)
                    if data.get("event") == "stopped":
                        self.stopped_events.append(data)
        except websockets.exceptions.ConnectionClosed:
            print("Connection closed")

    async def send_request(self, command: str, arguments: dict = None) -> dict:
        seq = self.seq
        self.seq += 1

        request = {
            "seq": seq,
            "type": "request",
            "command": command,
        }
        if arguments:
            request["arguments"] = arguments

        future = asyncio.Future()
        self.pending_requests[seq] = future

        print(f"--> Sending: {json.dumps(request, indent=2)}")
        await self.ws.send(json.dumps(request))

        # Wait for response with timeout
        try:
            response = await asyncio.wait_for(future, timeout=10.0)
            return response
        except asyncio.TimeoutError:
            print(f"Timeout waiting for response to {command}")
            raise

    async def initialize(self) -> dict:
        return await self.send_request("initialize", {
            "clientID": "test",
            "clientName": "DAP Test Client",
            "adapterID": "python",
            "pathFormat": "path",
            "linesStartAt1": True,
            "columnsStartAt1": True,
        })

    async def set_breakpoints(self, path: str, lines: list[int]) -> dict:
        return await self.send_request("setBreakpoints", {
            "source": {"path": path},
            "breakpoints": [{"line": line} for line in lines],
        })

    async def configuration_done(self) -> dict:
        return await self.send_request("configurationDone")

    async def launch(self, code: str, cwd: str = "/tmp", call_main: bool = False, args: dict = None) -> dict:
        return await self.send_request("launch", {
            "code": code,
            "cwd": cwd,
            "callMain": call_main,
            "args": args or {},
        })

    async def continue_(self) -> dict:
        return await self.send_request("continue", {"threadId": 1})

    async def get_stack_trace(self) -> dict:
        return await self.send_request("stackTrace", {
            "threadId": 1,
            "startFrame": 0,
            "levels": 20,
        })

    async def get_scopes(self, frame_id: int) -> dict:
        return await self.send_request("scopes", {"frameId": frame_id})

    async def get_variables(self, var_ref: int) -> dict:
        return await self.send_request("variables", {"variablesReference": var_ref})

    async def terminate(self) -> dict:
        return await self.send_request("terminate")

    async def wait_for_stopped(self, timeout: float = 5.0) -> dict:
        """Wait for a stopped event."""
        start = len(self.stopped_events)
        for _ in range(int(timeout * 10)):
            if len(self.stopped_events) > start:
                return self.stopped_events[-1]
            await asyncio.sleep(0.1)
        raise TimeoutError("No stopped event received")

    async def wait_for_event(self, event_name: str, timeout: float = 5.0) -> dict:
        """Wait for a specific event."""
        start = len(self.events)
        for _ in range(int(timeout * 10)):
            for event in self.events[start:]:
                if event.get("event") == event_name:
                    return event
            await asyncio.sleep(0.1)
        raise TimeoutError(f"No {event_name} event received")


async def run_test():
    client = DAPTestClient()

    try:
        await client.connect()
        await asyncio.sleep(0.1)  # Let receiver start

        # 1. Initialize
        print("\n=== STEP 1: Initialize ===")
        response = await client.initialize()
        assert response.get("success"), f"Initialize failed: {response}"
        print("Initialize: OK")

        # Wait for initialized event
        await asyncio.sleep(0.5)

        # 2. Set breakpoints
        print("\n=== STEP 2: Set Breakpoints ===")
        response = await client.set_breakpoints("/tmp/script.py", BREAKPOINT_LINES)
        assert response.get("success"), f"setBreakpoints failed: {response}"
        breakpoints = response.get("body", {}).get("breakpoints", [])
        print(f"Breakpoints set: {breakpoints}")
        assert len(breakpoints) == len(BREAKPOINT_LINES), "Wrong number of breakpoints"

        # 3. Configuration done
        print("\n=== STEP 3: Configuration Done ===")
        response = await client.configuration_done()
        assert response.get("success"), f"configurationDone failed: {response}"
        print("Configuration done: OK")

        # 4. Launch
        print("\n=== STEP 4: Launch ===")
        response = await client.launch(TEST_SCRIPT)
        assert response.get("success"), f"launch failed: {response}"
        print("Launch: OK")

        # 5. Wait for first breakpoint
        print("\n=== STEP 5: Wait for First Breakpoint ===")
        try:
            stopped = await client.wait_for_stopped(timeout=5.0)
            print(f"Stopped at: {stopped}")

            reason = stopped.get("body", {}).get("reason")
            print(f"Stop reason: {reason}")

            if reason == "breakpoint":
                print("SUCCESS: Hit first breakpoint!")
            else:
                print(f"WARNING: Stopped for reason '{reason}', not 'breakpoint'")

            # Get stack trace
            print("\n=== STEP 6: Get Stack Trace ===")
            stack_response = await client.get_stack_trace()
            frames = stack_response.get("body", {}).get("stackFrames", [])
            if frames:
                current_line = frames[0].get("line")
                print(f"Current line: {current_line}")
                if current_line in BREAKPOINT_LINES:
                    print(f"SUCCESS: Stopped at expected line {current_line}")
                else:
                    print(f"WARNING: Stopped at line {current_line}, expected one of {BREAKPOINT_LINES}")

            # Get variables
            print("\n=== STEP 7: Get Variables ===")
            scopes_response = await client.get_scopes(frames[0]["id"])
            scopes = scopes_response.get("body", {}).get("scopes", [])
            print(f"Scopes: {[s['name'] for s in scopes]}")

            if scopes:
                vars_response = await client.get_variables(scopes[0]["variablesReference"])
                variables = vars_response.get("body", {}).get("variables", [])
                print(f"Local variables: {[(v['name'], v['value']) for v in variables]}")

            # Continue to next breakpoint
            print("\n=== STEP 8: Continue to Next Breakpoint ===")
            await client.continue_()

            try:
                stopped = await client.wait_for_stopped(timeout=5.0)
                print(f"Stopped again at: {stopped}")

                stack_response = await client.get_stack_trace()
                frames = stack_response.get("body", {}).get("stackFrames", [])
                if frames:
                    current_line = frames[0].get("line")
                    print(f"Current line: {current_line}")

                    if current_line in BREAKPOINT_LINES:
                        print(f"SUCCESS: Hit second breakpoint at line {current_line}!")
                    else:
                        print(f"INFO: Stopped at line {current_line}")

                # Continue to end
                print("\n=== STEP 9: Continue to End ===")
                await client.continue_()

                # Wait for terminated event
                try:
                    await client.wait_for_event("terminated", timeout=5.0)
                    print("Script terminated normally")
                except TimeoutError:
                    print("Timeout waiting for terminated event")

            except TimeoutError:
                print("No second breakpoint hit - script may have ended")

        except TimeoutError:
            print("ERROR: No breakpoint was hit!")
            print("The script ran without stopping at breakpoints.")
            print("\nCheck server logs for:")
            print("  - 'Set breakpoint at' messages")
            print("  - 'user_line called' messages")
            print("  - 'Breakpoint HIT' messages")

        # Terminate
        print("\n=== STEP 10: Terminate ===")
        try:
            await client.terminate()
            print("Terminated: OK")
        except Exception as e:
            print(f"Terminate error (may be expected): {e}")

    except Exception as e:
        print(f"\nERROR: {e}")
        import traceback
        traceback.print_exc()
    finally:
        await client.disconnect()
        print("\n=== TEST COMPLETE ===")


async def run_main_test():
    """Test debugging a script with main() function (Windmill style)."""
    client = DAPTestClient()

    try:
        await client.connect()
        await asyncio.sleep(0.1)

        # 1. Initialize
        print("\n=== MAIN TEST: Initialize ===")
        response = await client.initialize()
        assert response.get("success"), f"Initialize failed: {response}"
        print("Initialize: OK")
        await asyncio.sleep(0.5)

        # 2. Set breakpoints inside main()
        print("\n=== MAIN TEST: Set Breakpoints ===")
        response = await client.set_breakpoints("/tmp/script.py", MAIN_BREAKPOINT_LINES)
        assert response.get("success"), f"setBreakpoints failed: {response}"
        print(f"Breakpoints set at lines: {MAIN_BREAKPOINT_LINES}")

        # 3. Configuration done
        print("\n=== MAIN TEST: Configuration Done ===")
        response = await client.configuration_done()
        assert response.get("success"), f"configurationDone failed: {response}"

        # 4. Launch with callMain=True and args
        print("\n=== MAIN TEST: Launch with callMain=True ===")
        test_args = {"x": "hello", "count": 3}
        response = await client.launch(
            TEST_SCRIPT_WITH_MAIN,
            call_main=True,
            args=test_args
        )
        assert response.get("success"), f"launch failed: {response}"
        print(f"Launch with args {test_args}: OK")

        # 5. Wait for breakpoint inside main()
        print("\n=== MAIN TEST: Wait for Breakpoint in main() ===")
        try:
            stopped = await client.wait_for_stopped(timeout=5.0)
            reason = stopped.get("body", {}).get("reason")
            print(f"Stopped! Reason: {reason}")

            # Get stack trace
            stack_response = await client.get_stack_trace()
            frames = stack_response.get("body", {}).get("stackFrames", [])
            if frames:
                current_line = frames[0].get("line")
                func_name = frames[0].get("name")
                print(f"Current location: {func_name}() at line {current_line}")

                if func_name == "main":
                    print("SUCCESS: Stopped inside main() function!")
                else:
                    print(f"INFO: Stopped in function '{func_name}'")

                # Get local variables to verify args were passed
                scopes_response = await client.get_scopes(frames[0]["id"])
                scopes = scopes_response.get("body", {}).get("scopes", [])
                if scopes:
                    vars_response = await client.get_variables(scopes[0]["variablesReference"])
                    variables = vars_response.get("body", {}).get("variables", [])
                    var_dict = {v["name"]: v["value"] for v in variables}
                    print(f"Variables: {var_dict}")

                    # Check if our args are present
                    if "x" in var_dict and "count" in var_dict:
                        print(f"SUCCESS: Args passed correctly! x={var_dict['x']}, count={var_dict['count']}")

            # Continue to end
            print("\n=== MAIN TEST: Continue to End ===")
            await client.continue_()

            # May hit another breakpoint or end
            try:
                stopped = await client.wait_for_stopped(timeout=2.0)
                print(f"Hit another breakpoint")
                await client.continue_()
            except TimeoutError:
                pass

            # Wait for output/terminated
            await asyncio.sleep(1.0)
            for event in client.events:
                if event.get("event") == "output":
                    output = event.get("body", {}).get("output", "")
                    print(f"Script output: {output}")

        except TimeoutError:
            print("ERROR: No breakpoint hit inside main()!")

        # Terminate
        print("\n=== MAIN TEST: Terminate ===")
        try:
            await client.terminate()
            print("Terminated: OK")
        except Exception as e:
            print(f"Terminate: {e}")

    except Exception as e:
        print(f"\nERROR: {e}")
        import traceback
        traceback.print_exc()
    finally:
        await client.disconnect()
        print("\n=== MAIN TEST COMPLETE ===")


# Test script with external import (requests)
TEST_SCRIPT_WITH_IMPORT = """
import requests

def main(url: str):
    response = requests.get(url)
    status = response.status_code
    print(f"Status: {status}")
    return {"status": status, "ok": response.ok}
"""


async def run_import_test():
    """Test that external dependencies are automatically installed."""
    print("\n" + "=" * 60)
    print("DYNAMIC IMPORT TEST")
    print("=" * 60)
    print("\nThis test verifies that external pip packages are automatically installed.")
    print("Make sure the server is started with: --windmill /path/to/windmill\n")

    client = DAPTestClient()

    try:
        await client.connect()

        # Initialize
        print("\n=== IMPORT TEST: Initialize ===")
        init_response = await client.initialize()
        if init_response.get("success"):
            print("Initialize: OK")
        else:
            print(f"Initialize: FAILED - {init_response}")
            return

        # Wait for initialized event
        await asyncio.sleep(0.5)

        # Launch with code that uses requests
        print("\n=== IMPORT TEST: Launch with requests import ===")
        launch_response = await client.launch(
            TEST_SCRIPT_WITH_IMPORT,
            call_main=True,
            args={"url": "https://httpbin.org/get"}
        )
        if launch_response.get("success"):
            print("Launch: OK")
        else:
            print(f"Launch: FAILED - {launch_response}")
            return

        # Wait for script to run and complete
        print("\n=== IMPORT TEST: Waiting for completion ===")
        terminated = False
        result = None
        timeout = 30  # 30 seconds for dependency installation + execution

        for _ in range(timeout * 10):  # Check every 100ms
            await asyncio.sleep(0.1)
            for event in client.events:
                if event.get("event") == "terminated":
                    terminated = True
                    result = event.get("body", {}).get("result")
                    break
            if terminated:
                break

        if not terminated:
            print("ERROR: Script did not terminate in time!")
            return

        # Check result
        print(f"\n=== IMPORT TEST: Result ===")
        print(f"Result: {result}")

        if result and result.get("status") == 200 and result.get("ok") is True:
            print("\nSUCCESS: External package (requests) was installed and worked correctly!")
        else:
            print(f"\nFAILED: Unexpected result - {result}")

        # Check output
        print("\n=== IMPORT TEST: Console Output ===")
        for event in client.events:
            if event.get("event") == "output":
                output = event.get("body", {}).get("output", "")
                print(f"  {output.strip()}")

        # Terminate
        print("\n=== IMPORT TEST: Terminate ===")
        try:
            await client.terminate()
            print("Terminated: OK")
        except Exception:
            pass  # May already be terminated

    except Exception as e:
        print(f"\nERROR: {e}")
        import traceback
        traceback.print_exc()
    finally:
        await client.disconnect()
        print("\n=== IMPORT TEST COMPLETE ===")


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--main", action="store_true", help="Run main() function test")
    parser.add_argument("--imports", action="store_true", help="Run dynamic imports test")
    parser.add_argument("--all", action="store_true", help="Run all tests")
    args = parser.parse_args()

    if args.main:
        asyncio.run(run_main_test())
    elif args.imports:
        asyncio.run(run_import_test())
    elif args.all:
        asyncio.run(run_test())
        print("\n" + "=" * 60 + "\n")
        asyncio.run(run_main_test())
        print("\n" + "=" * 60 + "\n")
        asyncio.run(run_import_test())
    else:
        asyncio.run(run_test())
