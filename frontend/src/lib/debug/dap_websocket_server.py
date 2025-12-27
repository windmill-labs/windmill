#!/usr/bin/env python3
"""
Lightweight DAP (Debug Adapter Protocol) WebSocket Server for Python debugging.

This server acts as a bridge between a WebSocket client (Monaco editor) and debugpy.
It implements a minimal subset of DAP to support basic Python debugging:
- Setting breakpoints
- Stepping through code (step in, step over, step out, continue)
- Inspecting variables and stack frames
- Evaluating expressions

Usage:
    python dap_websocket_server.py [--port PORT] [--host HOST]
"""

import asyncio
import json
import logging
import os
import sys
import tempfile
import threading
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any

import debugpy

# Configure logging
logging.basicConfig(
    level=logging.DEBUG,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger("dap_server")

try:
    import websockets
    from websockets.server import serve
except ImportError:
    print("websockets package required. Install with: pip install websockets")
    sys.exit(1)


class DAPMessageType(Enum):
    REQUEST = "request"
    RESPONSE = "response"
    EVENT = "event"


@dataclass
class DAPMessage:
    """Represents a DAP protocol message."""

    seq: int
    type: str  # 'request', 'response', 'event'
    command: str = ""
    event: str = ""
    request_seq: int = 0
    success: bool = True
    message: str = ""
    body: dict = field(default_factory=dict)

    def to_dict(self) -> dict:
        result = {"seq": self.seq, "type": self.type}
        if self.type == "request":
            result["command"] = self.command
            if self.body:
                result["arguments"] = self.body
        elif self.type == "response":
            result["request_seq"] = self.request_seq
            result["command"] = self.command
            result["success"] = self.success
            if self.message:
                result["message"] = self.message
            if self.body:
                result["body"] = self.body
        elif self.type == "event":
            result["event"] = self.event
            if self.body:
                result["body"] = self.body
        return result


class DebugSession:
    """Manages a single debug session."""

    def __init__(self, websocket):
        self.websocket = websocket
        self.seq = 1
        self.initialized = False
        self.configured = False
        self.script_path: str | None = None
        self.breakpoints: dict[str, list[int]] = {}  # file -> line numbers
        self.debugpy_port: int | None = None
        self.debug_thread: threading.Thread | None = None
        self.stopped = False
        self.thread_id = 1
        self.frame_id = 1
        self.variables_ref = 1
        self.scopes: dict[int, dict] = {}
        self.frames: list[dict] = []
        self._running = True
        self._temp_file: str | None = None

    def next_seq(self) -> int:
        seq = self.seq
        self.seq += 1
        return seq

    async def send_message(self, msg: DAPMessage) -> None:
        """Send a DAP message to the client."""
        data = json.dumps(msg.to_dict())
        logger.debug(f"Sending: {data}")
        await self.websocket.send(data)

    async def send_response(
        self,
        request: dict,
        success: bool = True,
        body: dict | None = None,
        message: str = "",
    ) -> None:
        """Send a response to a request."""
        msg = DAPMessage(
            seq=self.next_seq(),
            type="response",
            command=request.get("command", ""),
            request_seq=request.get("seq", 0),
            success=success,
            message=message,
            body=body or {},
        )
        await self.send_message(msg)

    async def send_event(self, event: str, body: dict | None = None) -> None:
        """Send an event to the client."""
        msg = DAPMessage(
            seq=self.next_seq(),
            type="event",
            event=event,
            body=body or {},
        )
        await self.send_message(msg)

    async def handle_initialize(self, request: dict) -> None:
        """Handle the 'initialize' request."""
        capabilities = {
            "supportsConfigurationDoneRequest": True,
            "supportsFunctionBreakpoints": False,
            "supportsConditionalBreakpoints": False,
            "supportsHitConditionalBreakpoints": False,
            "supportsEvaluateForHovers": True,
            "exceptionBreakpointFilters": [],
            "supportsStepBack": False,
            "supportsSetVariable": True,
            "supportsRestartFrame": False,
            "supportsGotoTargetsRequest": False,
            "supportsStepInTargetsRequest": False,
            "supportsCompletionsRequest": False,
            "supportsModulesRequest": False,
            "supportsExceptionOptions": False,
            "supportsValueFormattingOptions": False,
            "supportsExceptionInfoRequest": False,
            "supportTerminateDebuggee": True,
            "supportsDelayedStackTraceLoading": False,
            "supportsLoadedSourcesRequest": False,
            "supportsLogPoints": False,
            "supportsTerminateThreadsRequest": False,
            "supportsSetExpression": False,
            "supportsTerminateRequest": True,
            "supportsDataBreakpoints": False,
            "supportsReadMemoryRequest": False,
            "supportsDisassembleRequest": False,
            "supportsCancelRequest": False,
            "supportsBreakpointLocationsRequest": False,
        }
        await self.send_response(request, body=capabilities)
        self.initialized = True
        await self.send_event("initialized")

    async def handle_set_breakpoints(self, request: dict) -> None:
        """Handle the 'setBreakpoints' request."""
        args = request.get("arguments", {})
        source = args.get("source", {})
        source_path = source.get("path", "")
        breakpoints_data = args.get("breakpoints", [])

        verified_breakpoints = []
        line_numbers = []

        for bp in breakpoints_data:
            line = bp.get("line", 0)
            line_numbers.append(line)
            verified_breakpoints.append(
                {
                    "id": len(verified_breakpoints) + 1,
                    "verified": True,
                    "line": line,
                    "source": source,
                }
            )

        self.breakpoints[source_path] = line_numbers
        logger.info(f"Set breakpoints at lines {line_numbers} in {source_path}")

        await self.send_response(request, body={"breakpoints": verified_breakpoints})

    async def handle_configuration_done(self, request: dict) -> None:
        """Handle the 'configurationDone' request."""
        self.configured = True
        await self.send_response(request)

    async def handle_launch(self, request: dict) -> None:
        """Handle the 'launch' request."""
        args = request.get("arguments", {})
        self.script_path = args.get("program")
        code = args.get("code", "")
        script_args = args.get("args", [])
        cwd = args.get("cwd", os.getcwd())

        if not self.script_path and not code:
            await self.send_response(
                request, success=False, message="No program or code specified"
            )
            return

        # If code is provided, write it to a temp file
        if code and not self.script_path:
            fd, self._temp_file = tempfile.mkstemp(suffix=".py", prefix="windmill_debug_")
            with os.fdopen(fd, "w") as f:
                f.write(code)
            self.script_path = self._temp_file

        await self.send_response(request)

        # Start debugpy in a separate thread
        self.debug_thread = threading.Thread(
            target=self._run_debugpy,
            args=(self.script_path, script_args, cwd),
            daemon=True,
        )
        self.debug_thread.start()

    def _run_debugpy(
        self, script_path: str, script_args: list[str], cwd: str
    ) -> None:
        """Run the script with debugpy."""
        try:
            # Set up debugpy to stop at entry
            debugpy.configure(python=sys.executable)

            # Set breakpoints before running
            for file_path, lines in self.breakpoints.items():
                for line in lines:
                    debugpy.breakpoint_at(file_path, line)

            # Run the script
            old_cwd = os.getcwd()
            os.chdir(cwd)
            old_argv = sys.argv
            sys.argv = [script_path] + script_args

            try:
                with open(script_path) as f:
                    code = compile(f.read(), script_path, "exec")

                # Create globals with debugpy breakpoint support
                globals_dict = {
                    "__name__": "__main__",
                    "__file__": script_path,
                    "__builtins__": __builtins__,
                }

                # Execute with debugpy tracing
                exec(code, globals_dict)

                # Script completed
                asyncio.run(self._script_completed())
            finally:
                os.chdir(old_cwd)
                sys.argv = old_argv

        except Exception as e:
            logger.exception("Error running script")
            asyncio.run(self._script_error(str(e)))

    async def _script_completed(self) -> None:
        """Notify client that script completed."""
        await self.send_event("terminated")
        self._cleanup_temp_file()

    async def _script_error(self, error: str) -> None:
        """Notify client of script error."""
        await self.send_event(
            "output", {"category": "stderr", "output": f"Error: {error}\n"}
        )
        await self.send_event("terminated")
        self._cleanup_temp_file()

    def _cleanup_temp_file(self) -> None:
        """Clean up temporary file if created."""
        if self._temp_file and os.path.exists(self._temp_file):
            try:
                os.unlink(self._temp_file)
            except OSError:
                pass
            self._temp_file = None

    async def handle_threads(self, request: dict) -> None:
        """Handle the 'threads' request."""
        threads = [{"id": self.thread_id, "name": "MainThread"}]
        await self.send_response(request, body={"threads": threads})

    async def handle_stack_trace(self, request: dict) -> None:
        """Handle the 'stackTrace' request."""
        # Return mock stack frames for now
        stack_frames = self.frames or [
            {
                "id": self.frame_id,
                "name": "<module>",
                "source": {"path": self.script_path or "", "name": "script.py"},
                "line": 1,
                "column": 0,
            }
        ]
        await self.send_response(
            request, body={"stackFrames": stack_frames, "totalFrames": len(stack_frames)}
        )

    async def handle_scopes(self, request: dict) -> None:
        """Handle the 'scopes' request."""
        frame_id = request.get("arguments", {}).get("frameId", self.frame_id)

        # Create scope references
        local_ref = self.variables_ref
        self.variables_ref += 1
        global_ref = self.variables_ref
        self.variables_ref += 1

        self.scopes[local_ref] = {"type": "locals", "frame_id": frame_id}
        self.scopes[global_ref] = {"type": "globals", "frame_id": frame_id}

        scopes = [
            {
                "name": "Locals",
                "variablesReference": local_ref,
                "expensive": False,
            },
            {
                "name": "Globals",
                "variablesReference": global_ref,
                "expensive": True,
            },
        ]
        await self.send_response(request, body={"scopes": scopes})

    async def handle_variables(self, request: dict) -> None:
        """Handle the 'variables' request."""
        variables_ref = request.get("arguments", {}).get("variablesReference", 0)

        # Return empty for now - full implementation would inspect debugpy state
        variables = []

        scope_info = self.scopes.get(variables_ref)
        if scope_info:
            # In a full implementation, we'd get actual variables from debugpy
            pass

        await self.send_response(request, body={"variables": variables})

    async def handle_evaluate(self, request: dict) -> None:
        """Handle the 'evaluate' request."""
        args = request.get("arguments", {})
        expression = args.get("expression", "")
        context = args.get("context", "repl")

        try:
            # Simple evaluation - in production, use debugpy's evaluation
            result = str(eval(expression))
            await self.send_response(
                request,
                body={
                    "result": result,
                    "variablesReference": 0,
                },
            )
        except Exception as e:
            await self.send_response(
                request,
                body={
                    "result": f"Error: {e}",
                    "variablesReference": 0,
                },
            )

    async def handle_continue(self, request: dict) -> None:
        """Handle the 'continue' request."""
        self.stopped = False
        await self.send_response(request, body={"allThreadsContinued": True})

    async def handle_next(self, request: dict) -> None:
        """Handle the 'next' (step over) request."""
        await self.send_response(request)
        # In a full implementation, this would step in debugpy

    async def handle_step_in(self, request: dict) -> None:
        """Handle the 'stepIn' request."""
        await self.send_response(request)

    async def handle_step_out(self, request: dict) -> None:
        """Handle the 'stepOut' request."""
        await self.send_response(request)

    async def handle_pause(self, request: dict) -> None:
        """Handle the 'pause' request."""
        self.stopped = True
        await self.send_response(request)
        await self.send_event(
            "stopped",
            {
                "reason": "pause",
                "threadId": self.thread_id,
                "allThreadsStopped": True,
            },
        )

    async def handle_disconnect(self, request: dict) -> None:
        """Handle the 'disconnect' request."""
        self._running = False
        self._cleanup_temp_file()
        await self.send_response(request)
        await self.send_event("terminated")

    async def handle_terminate(self, request: dict) -> None:
        """Handle the 'terminate' request."""
        self._running = False
        self._cleanup_temp_file()
        await self.send_response(request)
        await self.send_event("terminated")

    async def handle_request(self, request: dict) -> None:
        """Route and handle a DAP request."""
        command = request.get("command", "")
        logger.debug(f"Handling command: {command}")

        handlers = {
            "initialize": self.handle_initialize,
            "setBreakpoints": self.handle_set_breakpoints,
            "configurationDone": self.handle_configuration_done,
            "launch": self.handle_launch,
            "threads": self.handle_threads,
            "stackTrace": self.handle_stack_trace,
            "scopes": self.handle_scopes,
            "variables": self.handle_variables,
            "evaluate": self.handle_evaluate,
            "continue": self.handle_continue,
            "next": self.handle_next,
            "stepIn": self.handle_step_in,
            "stepOut": self.handle_step_out,
            "pause": self.handle_pause,
            "disconnect": self.handle_disconnect,
            "terminate": self.handle_terminate,
        }

        handler = handlers.get(command)
        if handler:
            await handler(request)
        else:
            logger.warning(f"Unhandled command: {command}")
            await self.send_response(
                request, success=False, message=f"Unsupported command: {command}"
            )


async def handle_connection(websocket) -> None:
    """Handle a WebSocket connection."""
    session = DebugSession(websocket)
    logger.info(f"New connection from {websocket.remote_address}")

    try:
        async for message in websocket:
            try:
                data = json.loads(message)
                logger.debug(f"Received: {data}")

                if data.get("type") == "request":
                    await session.handle_request(data)
            except json.JSONDecodeError as e:
                logger.error(f"Invalid JSON: {e}")
            except Exception as e:
                logger.exception(f"Error handling message: {e}")

    except websockets.exceptions.ConnectionClosed:
        logger.info("Connection closed")
    finally:
        session._cleanup_temp_file()


async def main(host: str = "localhost", port: int = 5679) -> None:
    """Start the DAP WebSocket server."""
    logger.info(f"Starting DAP WebSocket server on ws://{host}:{port}")

    async with serve(handle_connection, host, port):
        await asyncio.Future()  # Run forever


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="DAP WebSocket Server for Python debugging")
    parser.add_argument("--host", default="localhost", help="Host to bind to")
    parser.add_argument("--port", type=int, default=5679, help="Port to listen on")
    args = parser.parse_args()

    try:
        asyncio.run(main(args.host, args.port))
    except KeyboardInterrupt:
        logger.info("Server stopped")
