"""Tests for the Workflow-as-Code SDK."""

import asyncio
import pytest

from wmill.client import WorkflowCtx, _StepSuspend, workflow, task, _run_workflow


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
