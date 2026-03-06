"""Tests for the Workflow-as-Code SDK."""

import asyncio
import pytest
import time

from wmill.client import WorkflowCtx, _StepSuspend, workflow, task, step, _run_workflow


@task
async def extract_data(url: str):
    pass  # body unused in workflow context


@task
async def load_data(data=None):
    pass


@task
async def clean_data(data=None):
    pass


@task
async def compute_stats(data=None):
    pass


@task
async def send_alert(msg: str = ""):
    pass


@task
async def double(x: int):
    return x * 2


@workflow
async def simple_workflow(url: str):
    raw = await extract_data(url=url)
    result = await load_data(data=raw)
    return {"status": "done", "result": result}


@workflow
async def parallel_workflow(url: str):
    raw = await extract_data(url=url)
    cleaned, stats = await asyncio.gather(
        clean_data(data=raw),
        compute_stats(data=raw),
    )
    return {"cleaned": cleaned, "stats": stats}


@workflow
async def conditional_workflow(count: int):
    if count > 100:
        await send_alert(msg="large")
    await load_data()
    return {"done": True}


@workflow
async def step_workflow(x: int):
    ts = await step("timestamp", lambda: 1234567890)
    doubled = await double(x=x)
    rid = await step("random_id", lambda: "abc-123")
    return {"ts": ts, "doubled": doubled, "id": rid}


class TestWorkflowDecorator:
    def test_marks_function(self):
        assert hasattr(simple_workflow, "_is_workflow")
        assert simple_workflow._is_workflow is True


class TestTaskDecorator:
    def test_marks_function(self):
        assert hasattr(extract_data, "_is_task")
        assert extract_data._is_task is True

    def test_standalone_execution(self):
        """Outside a workflow, @task runs the function body directly."""
        result = asyncio.run(extract_data(url="https://example.com"))
        assert result is None  # body returns None


class TestFirstInvocation:
    def test_dispatches_first_step(self):
        result = _run_workflow(simple_workflow, {}, {"url": "https://example.com"})
        assert result["type"] == "dispatch"
        assert result["mode"] == "sequential"
        assert len(result["steps"]) == 1
        assert result["steps"][0]["name"] == "extract_data"
        assert result["steps"][0]["script"] == "extract_data"
        assert result["steps"][0]["key"] == "step_0"
        assert result["steps"][0]["args"] == {"url": "https://example.com"}


class TestReplayWithCheckpoint:
    def test_second_invocation_dispatches_second_step(self):
        checkpoint = {
            "completed_steps": {
                "step_0": {"data": [1, 2, 3]},
            }
        }
        result = _run_workflow(simple_workflow, checkpoint, {"url": "https://example.com"})
        assert result["type"] == "dispatch"
        assert result["mode"] == "sequential"
        assert result["steps"][0]["name"] == "load_data"
        assert result["steps"][0]["key"] == "step_1"

    def test_all_steps_complete(self):
        checkpoint = {
            "completed_steps": {
                "step_0": {"data": [1, 2, 3]},
                "step_1": {"loaded": True},
            }
        }
        result = _run_workflow(simple_workflow, checkpoint, {"url": "https://example.com"})
        assert result["type"] == "complete"
        assert result["result"]["status"] == "done"
        assert result["result"]["result"] == {"loaded": True}


class TestParallelDispatch:
    def test_first_invocation(self):
        result = _run_workflow(parallel_workflow, {}, {"url": "https://example.com"})
        assert result["type"] == "dispatch"
        assert result["steps"][0]["name"] == "extract_data"

    def test_parallel_dispatch(self):
        checkpoint = {
            "completed_steps": {
                "step_0": {"raw": "data"},
            }
        }
        result = _run_workflow(parallel_workflow, checkpoint, {"url": "https://example.com"})
        assert result["type"] == "dispatch"
        assert result["mode"] == "parallel"
        assert len(result["steps"]) == 2
        assert result["steps"][0]["name"] == "clean_data"
        assert result["steps"][1]["name"] == "compute_stats"

    def test_parallel_complete(self):
        checkpoint = {
            "completed_steps": {
                "step_0": {"raw": "data"},
                "step_1": {"cleaned": True},
                "step_2": {"count": 42},
            }
        }
        result = _run_workflow(parallel_workflow, checkpoint, {"url": "https://example.com"})
        assert result["type"] == "complete"
        assert result["result"]["cleaned"] == {"cleaned": True}
        assert result["result"]["stats"] == {"count": 42}


class TestConditionalWorkflow:
    def test_condition_true(self):
        checkpoint = {}
        result = _run_workflow(conditional_workflow, checkpoint, {"count": 200})
        # Should dispatch notify step first
        assert result["type"] == "dispatch"
        assert result["steps"][0]["name"] == "send_alert"

    def test_condition_false(self):
        checkpoint = {}
        result = _run_workflow(conditional_workflow, checkpoint, {"count": 50})
        # Should skip notify and dispatch load
        assert result["type"] == "dispatch"
        assert result["steps"][0]["name"] == "load_data"


class TestStepInlineCheckpoint:
    def test_first_invocation_returns_inline_checkpoint(self):
        result = _run_workflow(step_workflow, {}, {"x": 7})
        assert result["type"] == "inline_checkpoint"
        assert result["key"] == "step_0"
        assert result["result"] == 1234567890

    def test_step_cached_then_task_dispatches(self):
        checkpoint = {
            "completed_steps": {
                "step_0": 1234567890,
            }
        }
        result = _run_workflow(step_workflow, checkpoint, {"x": 7})
        assert result["type"] == "dispatch"
        assert result["mode"] == "sequential"
        assert result["steps"][0]["name"] == "double"
        assert result["steps"][0]["key"] == "step_1"

    def test_step_and_task_cached_then_second_step(self):
        checkpoint = {
            "completed_steps": {
                "step_0": 1234567890,
                "step_1": 14,
            }
        }
        result = _run_workflow(step_workflow, checkpoint, {"x": 7})
        assert result["type"] == "inline_checkpoint"
        assert result["key"] == "step_2"
        assert result["result"] == "abc-123"

    def test_all_complete(self):
        checkpoint = {
            "completed_steps": {
                "step_0": 1234567890,
                "step_1": 14,
                "step_2": "abc-123",
            }
        }
        result = _run_workflow(step_workflow, checkpoint, {"x": 7})
        assert result["type"] == "complete"
        assert result["result"] == {"ts": 1234567890, "doubled": 14, "id": "abc-123"}


class TestChildMode:
    def test_child_executes_matching_task(self):
        checkpoint = {
            "completed_steps": {
                "step_0": 1234567890,
            },
            "_executing_key": "step_1",
        }
        result = _run_workflow(step_workflow, checkpoint, {"x": 7})
        assert result["type"] == "complete"
        assert result["result"] == 14  # double(7) = 14

    def test_child_replays_cached_steps(self):
        """Cached steps before the executing key should replay normally."""
        checkpoint = {
            "completed_steps": {
                "step_0": {"data": [1, 2, 3]},
            },
            "_executing_key": "step_1",
        }
        result = _run_workflow(simple_workflow, checkpoint, {"url": "https://example.com"})
        assert result["type"] == "complete"
        # load_data body returns None
        assert result["result"] is None
