## Python Workflow-as-Code API (wmill)

Import: `from wmill import workflow, task, task_script, task_flow, step, sleep, wait_for_approval, get_resume_urls, parallel, TaskError`

```python
# Raised when a WAC task step failed.
#
# Attributes:
#     step_key: The checkpoint key of the failed step.
#     child_job_id: The UUID of the failed child job.
#     result: The error result from the child job.
class TaskError(Exception):
    def __init__(self, message: str, *, step_key: str = '', child_job_id: str = '', result = None)

# Get URLs needed for resuming a flow after suspension.
#
# Args:
#     approver: Optional approver name
#     flow_level: If True, generate resume URLs for the parent flow instead of the
#         specific step. This allows pre-approvals that can be consumed by any later
#         suspend step in the same flow.
#
# Returns:
#     Dictionary with approvalPage, resume, and cancel URLs
def get_resume_urls(approver: str = None, flow_level: bool = None) -> dict

# Decorator that marks a function as a workflow task.
#
# Works in both WAC v1 (sync, HTTP-based dispatch) and WAC v2
# (async, checkpoint/replay) modes:
#
# - **v2 (inside @workflow)**: dispatches as a checkpoint step.
# - **v1 (WM_JOB_ID set, no @workflow)**: dispatches via HTTP API.
# - **Standalone**: executes the function body directly.
#
# Usage::
#
#     @task
#     async def extract_data(url: str): ...
#
#     @task(path="f/external_script", timeout=600, tag="gpu")
#     async def run_external(x: int): ...
def task(_func = None, *, path: Optional[str] = None, tag: Optional[str] = None, timeout: Optional[int] = None, cache_ttl: Optional[int] = None, priority: Optional[int] = None, concurrency_limit: Optional[int] = None, concurrency_key: Optional[str] = None, concurrency_time_window_s: Optional[int] = None)

# Create a task that dispatches to a separate Windmill script.
#
# Usage::
#
#     extract = task_script("f/data/extract", timeout=600)
#
#     @workflow
#     async def main():
#         data = await extract(url="https://...")
def task_script(path: str, *, timeout: Optional[int] = None, tag: Optional[str] = None, cache_ttl: Optional[int] = None, priority: Optional[int] = None, concurrency_limit: Optional[int] = None, concurrency_key: Optional[str] = None, concurrency_time_window_s: Optional[int] = None)

# Create a task that dispatches to a separate Windmill flow.
#
# Usage::
#
#     pipeline = task_flow("f/etl/pipeline", priority=10)
#
#     @workflow
#     async def main():
#         result = await pipeline(input=data)
def task_flow(path: str, *, timeout: Optional[int] = None, tag: Optional[str] = None, cache_ttl: Optional[int] = None, priority: Optional[int] = None, concurrency_limit: Optional[int] = None, concurrency_key: Optional[str] = None, concurrency_time_window_s: Optional[int] = None)

# Decorator marking an async function as a workflow-as-code entry point.
#
# The function must be **deterministic**: given the same inputs it must call
# tasks in the same order on every replay. Branching on task results is fine
# (results are replayed from checkpoint), but branching on external state
# (current time, random values, external API calls) must use ``step()`` to
# checkpoint the value so replays see the same result.
def workflow(func)

# Execute ``fn`` inline and checkpoint the result.
#
# On replay the cached value is returned without re-executing ``fn``.
# Use for lightweight deterministic operations (timestamps, random IDs,
# config reads) that should not incur the overhead of a child job.
async def step(name: str, fn)

# Server-side sleep — suspend the workflow for the given duration without holding a worker.
#
# Inside a @workflow, the parent job suspends and auto-resumes after ``seconds``.
# Outside a workflow, falls back to ``asyncio.sleep``.
async def sleep(seconds: int)

# Suspend the workflow and wait for an external approval.
#
# Use ``get_resume_urls()`` (wrapped in ``step()``) to obtain
# resume/cancel/approval URLs before calling this function.
#
# Returns a dict with ``value`` (form data), ``approver``, and ``approved``.
#
# Args:
#     timeout: Approval timeout in seconds (default 1800).
#     form: Optional form schema for the approval page.
#     self_approval: Whether the user who triggered the flow can approve it (default True).
#
# Example::
#
#     urls = await step("urls", lambda: get_resume_urls())
#     await step("notify", lambda: send_email(urls["approvalPage"]))
#     result = await wait_for_approval(timeout=3600)
async def wait_for_approval(timeout: int = 1800, form: dict | None = None, self_approval: bool = True) -> dict

# Process items in parallel with optional concurrency control.
#
# Each item is processed by calling ``fn(item)``, which should be a @task.
# Items are dispatched in batches of ``concurrency`` (default: all at once).
#
# Example::
#
#     @task
#     async def process(item: str):
#         ...
#
#     results = await parallel(items, process, concurrency=5)
async def parallel(items, fn, *, concurrency: Optional[int] = None)
```
