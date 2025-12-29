#!/usr/bin/env python3
"""
Lightweight DAP (Debug Adapter Protocol) WebSocket Server for Python debugging.

This server acts as a bridge between a WebSocket client (Monaco editor) and Python's
built-in debugging capabilities using the bdb module.

It implements a minimal subset of DAP to support basic Python debugging:
- Setting breakpoints
- Stepping through code (step in, step over, step out, continue)
- Inspecting variables and stack frames
- Evaluating expressions

Usage:
    python dap_websocket_server.py [--port PORT] [--host HOST]
"""

import asyncio
import bdb
import json
import linecache
import logging
import os
import sys
import tempfile
import threading
import traceback
from dataclasses import dataclass, field
from enum import Enum
from io import StringIO
from typing import Any

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


class WindmillDebugger(bdb.Bdb):
    """A debugger based on Python's bdb module."""

    def __init__(self, session: "DebugSession"):
        super().__init__()
        self.session = session
        self.main_thread = threading.current_thread()
        self._wait_for_continue = threading.Event()
        self._step_mode = None  # None, 'over', 'in', 'out'
        self._stop_requested = False
        self._current_frame = None
        self._loop = None

    def user_line(self, frame):
        """Called when we stop at a line."""
        if self._stop_requested:
            raise bdb.BdbQuit()

        self._current_frame = frame
        filename = self.canonic(frame.f_code.co_filename)
        lineno = frame.f_lineno

        logger.debug(f"user_line called: {filename}:{lineno}, breaks={self.get_all_breaks()}")

        # Check if we should stop here
        should_stop = False
        reason = "step"

        # Check breakpoints using bdb's built-in method
        if self.break_here(frame):
            should_stop = True
            reason = "breakpoint"
            logger.info(f"Breakpoint HIT at {filename}:{lineno}")
        elif self._step_mode == 'in':
            should_stop = True
            reason = "step"
        elif self._step_mode == 'over':
            should_stop = True
            reason = "step"
        elif self._step_mode == 'out':
            # Will be handled by user_return
            pass

        if should_stop:
            self._step_mode = None
            # Notify the client that we've stopped
            asyncio.run_coroutine_threadsafe(
                self.session.send_event(
                    "stopped",
                    {
                        "reason": reason,
                        "threadId": 1,
                        "allThreadsStopped": True,
                    },
                ),
                self._loop,
            )
            # Wait for continue/step command
            self._wait_for_continue.clear()
            self._wait_for_continue.wait()

            if self._stop_requested:
                raise bdb.BdbQuit()

    def user_return(self, frame, return_value):
        """Called when a return is about to happen."""
        if self._step_mode == 'out':
            self._step_mode = None
            self._current_frame = frame
            asyncio.run_coroutine_threadsafe(
                self.session.send_event(
                    "stopped",
                    {
                        "reason": "step",
                        "threadId": 1,
                        "allThreadsStopped": True,
                    },
                ),
                self._loop,
            )
            self._wait_for_continue.clear()
            self._wait_for_continue.wait()

    def user_exception(self, frame, exc_info):
        """Called when an exception occurs."""
        exc_type, exc_value, exc_tb = exc_info
        self._current_frame = frame
        asyncio.run_coroutine_threadsafe(
            self.session.send_event(
                "stopped",
                {
                    "reason": "exception",
                    "threadId": 1,
                    "allThreadsStopped": True,
                    "text": str(exc_value),
                },
            ),
            self._loop,
        )
        self._wait_for_continue.clear()
        self._wait_for_continue.wait()

    def do_continue(self):
        """Continue execution."""
        self._step_mode = None
        self._wait_for_continue.set()

    def do_step_over(self):
        """Step over (next line)."""
        self._step_mode = 'over'
        self.set_next(self._current_frame)
        self._wait_for_continue.set()

    def do_step_in(self):
        """Step into."""
        self._step_mode = 'in'
        self.set_step()
        self._wait_for_continue.set()

    def do_step_out(self):
        """Step out."""
        self._step_mode = 'out'
        self.set_return(self._current_frame)
        self._wait_for_continue.set()

    def do_stop(self):
        """Stop debugging."""
        self._stop_requested = True
        self._wait_for_continue.set()

    def get_stack_frames(self) -> list[dict]:
        """Get current stack frames, stopping at <module> (user's script entry point)."""
        frames = []
        if self._current_frame is None:
            return frames

        frame = self._current_frame
        frame_id = 1
        while frame is not None:
            filename = self.canonic(frame.f_code.co_filename)
            name = frame.f_code.co_name
            frames.append({
                "id": frame_id,
                "name": name,
                "source": {"path": filename, "name": os.path.basename(filename)},
                "line": frame.f_lineno,
                "column": 0,
            })
            # Stop at <module> - don't include debugger/threading internals
            if name == "<module>":
                break
            frame = frame.f_back
            frame_id += 1
        return frames

    def get_frame_by_id(self, frame_id: int):
        """Get a frame by its ID."""
        frame = self._current_frame
        current_id = 1
        while frame is not None:
            if current_id == frame_id:
                return frame
            frame = frame.f_back
            current_id += 1
        return None

    def get_locals(self, frame_id: int = 1) -> dict:
        """Get local variables for a frame."""
        frame = self.get_frame_by_id(frame_id)
        if frame:
            return frame.f_locals.copy()
        return {}

    def get_globals(self, frame_id: int = 1) -> dict:
        """Get global variables for a frame."""
        frame = self.get_frame_by_id(frame_id)
        if frame:
            return frame.f_globals.copy()
        return {}


class DebugSession:
    """Manages a single debug session."""

    def __init__(self, websocket):
        self.websocket = websocket
        self.seq = 1
        self.initialized = False
        self.configured = False
        self.script_path: str | None = None
        self.breakpoints: dict[str, list[int]] = {}  # file -> line numbers
        self.debug_thread: threading.Thread | None = None
        self.debugger: WindmillDebugger | None = None
        self._running = True
        self._temp_file: str | None = None
        self._variables_ref_counter = 1
        self._scopes_map: dict[int, dict] = {}  # ref -> {type, frame_id}
        self._loop = asyncio.get_event_loop()
        self._call_main = False
        self._main_args: dict = {}

    def next_seq(self) -> int:
        seq = self.seq
        self.seq += 1
        return seq

    def _next_var_ref(self) -> int:
        ref = self._variables_ref_counter
        self._variables_ref_counter += 1
        return ref

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
            "supportsSetVariable": False,
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

        # Store breakpoints - they'll be applied when launch is called
        self.breakpoints[source_path] = line_numbers
        logger.info(f"Stored breakpoints at lines {line_numbers} for {source_path}")

        # If debugger already exists and we have a script path, update breakpoints now
        if self.debugger and self.script_path:
            self.debugger.clear_all_breaks()
            for line in line_numbers:
                self.debugger.set_break(self.script_path, line)
                logger.info(f"Updated breakpoint at {self.script_path}:{line}")

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
        cwd = args.get("cwd", os.getcwd())
        self._call_main = args.get("callMain", False)
        self._main_args = args.get("args", {})

        if not self.script_path and not code:
            await self.send_response(
                request, success=False, message="No program or code specified"
            )
            return

        # If callMain is True, append a call to main() with the provided args
        if self._call_main and code:
            # Generate the main() call with kwargs
            args_str = ", ".join(f"{k}={repr(v)}" for k, v in self._main_args.items())
            code = code + f"\n\n# Auto-generated call to main entrypoint\n__windmill_result__ = main({args_str})\n"
            logger.info(f"Added main() call with args: {args_str}")

        # If code is provided, write it to a temp file
        if code and not self.script_path:
            fd, self._temp_file = tempfile.mkstemp(suffix=".py", prefix="windmill_debug_")
            with os.fdopen(fd, "w") as f:
                f.write(code)
            self.script_path = self._temp_file

        await self.send_response(request)

        # Create debugger
        self.debugger = WindmillDebugger(self)
        self.debugger._loop = self._loop

        # Set breakpoints in the debugger using the actual script path
        # (breakpoints from frontend may use a different path like /tmp/script.py)
        self.debugger.clear_all_breaks()
        canonical_path = self.debugger.canonic(self.script_path)
        logger.info(f"Script path: {self.script_path}, canonical: {canonical_path}")
        logger.info(f"Stored breakpoints from frontend: {self.breakpoints}")

        for file_path, lines in self.breakpoints.items():
            logger.info(f"Processing breakpoints for frontend path '{file_path}': lines {lines}")
            for line in lines:
                # Use the actual script path, not the frontend path
                error = self.debugger.set_break(self.script_path, line)
                if error:
                    logger.error(f"Failed to set breakpoint at {self.script_path}:{line}: {error}")
                else:
                    logger.info(f"Set breakpoint at {self.script_path}:{line}")

        # Log all registered breakpoints for debugging
        logger.info(f"Debugger breaks after setup: {self.debugger.get_all_breaks()}")

        # Start debugging in a separate thread
        self.debug_thread = threading.Thread(
            target=self._run_script,
            args=(self.script_path, cwd),
            daemon=True,
        )
        self.debug_thread.start()

    def _run_script(self, script_path: str, cwd: str) -> None:
        """Run the script with the debugger."""
        old_cwd = os.getcwd()
        old_argv = sys.argv
        old_stdout = sys.stdout
        old_stderr = sys.stderr

        # Create a streaming output wrapper that sends output events in real-time
        session = self
        loop = self._loop

        class StreamingOutput:
            def __init__(self, category: str):
                self.category = category
                self.buffer = ""

            def write(self, data: str) -> int:
                if data:
                    # Send output event immediately
                    asyncio.run_coroutine_threadsafe(
                        session.send_event("output", {"category": self.category, "output": data}),
                        loop,
                    )
                return len(data)

            def flush(self):
                pass

        streaming_stdout = StreamingOutput("stdout")
        streaming_stderr = StreamingOutput("stderr")

        try:
            os.chdir(cwd)
            sys.argv = [script_path]
            sys.stdout = streaming_stdout
            sys.stderr = streaming_stderr

            # Read and compile the script
            with open(script_path) as f:
                code = f.read()

            logger.info(f"Running script: {script_path}")
            logger.info(f"Script content ({len(code)} chars):\n{code[:500]}...")

            # Clear linecache to ensure fresh source
            linecache.checkcache(script_path)

            compiled = compile(code, script_path, "exec")
            logger.info(f"Compiled code filename: {compiled.co_filename}")

            # Create globals
            globals_dict = {
                "__name__": "__main__",
                "__file__": script_path,
                "__builtins__": __builtins__,
            }

            # Run with debugger
            logger.info(f"Starting debugger.run() with breaks: {self.debugger.get_all_breaks()}")
            self.debugger.run(compiled, globals_dict)
            logger.info("debugger.run() completed normally")

            # Script completed normally - get the result from main()
            result = globals_dict.get("__windmill_result__")
            logger.info(f"Script result: {result}")

            asyncio.run_coroutine_threadsafe(
                self.send_event("terminated", {"result": result}),
                self._loop,
            )

        except bdb.BdbQuit:
            # Normal termination via stop
            asyncio.run_coroutine_threadsafe(
                self.send_event("terminated"),
                self._loop,
            )
        except Exception as e:
            error_msg = traceback.format_exc()
            logger.exception("Error running script")
            asyncio.run_coroutine_threadsafe(
                self.send_event("output", {"category": "stderr", "output": error_msg}),
                self._loop,
            )
            asyncio.run_coroutine_threadsafe(
                self.send_event("terminated", {"error": str(e)}),
                self._loop,
            )
        finally:
            os.chdir(old_cwd)
            sys.argv = old_argv
            sys.stdout = old_stdout
            sys.stderr = old_stderr
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
        threads = [{"id": 1, "name": "MainThread"}]
        await self.send_response(request, body={"threads": threads})

    async def handle_stack_trace(self, request: dict) -> None:
        """Handle the 'stackTrace' request."""
        if self.debugger:
            stack_frames = self.debugger.get_stack_frames()
        else:
            stack_frames = []
        await self.send_response(
            request, body={"stackFrames": stack_frames, "totalFrames": len(stack_frames)}
        )

    async def handle_scopes(self, request: dict) -> None:
        """Handle the 'scopes' request."""
        frame_id = request.get("arguments", {}).get("frameId", 1)

        # Create scope references
        local_ref = self._next_var_ref()
        global_ref = self._next_var_ref()

        self._scopes_map[local_ref] = {"type": "locals", "frame_id": frame_id}
        self._scopes_map[global_ref] = {"type": "globals", "frame_id": frame_id}

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
        variables = []

        scope_info = self._scopes_map.get(variables_ref)
        if scope_info and self.debugger:
            frame_id = scope_info["frame_id"]
            if scope_info["type"] == "locals":
                var_dict = self.debugger.get_locals(frame_id)
            else:
                var_dict = self.debugger.get_globals(frame_id)

            for name, value in var_dict.items():
                # Skip private/magic attributes for globals
                if scope_info["type"] == "globals" and name.startswith("_"):
                    continue
                try:
                    value_str = repr(value)
                    if len(value_str) > 100:
                        value_str = value_str[:97] + "..."
                    variables.append({
                        "name": name,
                        "value": value_str,
                        "type": type(value).__name__,
                        "variablesReference": 0,
                    })
                except Exception:
                    variables.append({
                        "name": name,
                        "value": "<error getting value>",
                        "type": "unknown",
                        "variablesReference": 0,
                    })

        await self.send_response(request, body={"variables": variables})

    async def handle_evaluate(self, request: dict) -> None:
        """Handle the 'evaluate' request."""
        args = request.get("arguments", {})
        expression = args.get("expression", "")
        frame_id = args.get("frameId", 1)

        try:
            if self.debugger:
                frame = self.debugger.get_frame_by_id(frame_id)
                if frame:
                    result = eval(expression, frame.f_globals, frame.f_locals)
                    result_str = repr(result)
                else:
                    result_str = "<no frame>"
            else:
                result_str = eval(expression)
                result_str = repr(result_str)

            await self.send_response(
                request,
                body={
                    "result": result_str,
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
        if self.debugger:
            self.debugger.do_continue()
        await self.send_response(request, body={"allThreadsContinued": True})

    async def handle_next(self, request: dict) -> None:
        """Handle the 'next' (step over) request."""
        if self.debugger:
            self.debugger.do_step_over()
        await self.send_response(request)

    async def handle_step_in(self, request: dict) -> None:
        """Handle the 'stepIn' request."""
        if self.debugger:
            self.debugger.do_step_in()
        await self.send_response(request)

    async def handle_step_out(self, request: dict) -> None:
        """Handle the 'stepOut' request."""
        if self.debugger:
            self.debugger.do_step_out()
        await self.send_response(request)

    async def handle_pause(self, request: dict) -> None:
        """Handle the 'pause' request."""
        await self.send_response(request)
        await self.send_event(
            "stopped",
            {
                "reason": "pause",
                "threadId": 1,
                "allThreadsStopped": True,
            },
        )

    async def handle_disconnect(self, request: dict) -> None:
        """Handle the 'disconnect' request."""
        self._running = False
        if self.debugger:
            self.debugger.do_stop()
        self._cleanup_temp_file()
        await self.send_response(request)

    async def handle_terminate(self, request: dict) -> None:
        """Handle the 'terminate' request."""
        self._running = False
        if self.debugger:
            self.debugger.do_stop()
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
        if session.debugger:
            session.debugger.do_stop()
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
