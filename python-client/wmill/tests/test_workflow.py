"""Tests for the Workflow-as-Code SDK."""

import asyncio
import pytest

from wmill.client import WorkflowCtx, _StepSuspend, TaskError, workflow, task, step, sleep, parallel, _run_workflow


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


@task
async def add_one(x: int):
    return x + 1


@task
async def noop_task():
    pass


# --- Module-level workflow definitions ---

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


# Edge case workflows

@workflow
async def three_step_wf(n: int):
    doubled = await double(x=n)
    incremented = await add_one(x=doubled)
    final = await double(x=incremented)
    return {"doubled": doubled, "incremented": incremented, "final": final}


@workflow
async def seq_par_seq_wf(url: str):
    raw = await extract_data(url=url)
    cleaned, stats = await asyncio.gather(
        clean_data(data=raw),
        compute_stats(data=raw),
    )
    loaded = await load_data(data={"cleaned": cleaned, "stats": stats})
    return loaded


@workflow
async def double_parallel_wf():
    a, b = await asyncio.gather(double(x=1), double(x=2))
    c, d = await asyncio.gather(add_one(x=a), add_one(x=b))
    return {"a": a, "b": b, "c": c, "d": d}


@workflow
async def cond_on_result_wf():
    val = await double(x=5)
    if val > 8:
        await send_alert(msg="big")
    await load_data(data=val)
    return {"val": val}


@workflow
async def empty_wf():
    return {"status": "empty"}


@workflow
async def single_wf(x: int):
    result = await double(x=x)
    return result


@workflow
async def no_arg_wf():
    result = await noop_task()
    return result


@workflow
async def many_steps_wf(n: int):
    val = n
    for _ in range(10):
        val = await add_one(x=val)
    return val


@workflow
async def falsy_wf():
    a = await double(x=0)
    b = await load_data(data=a)
    c = await extract_data(url="")
    return {"a": a, "b": b, "c": c}


@task(path="f/external_script")
async def run_external(x: int):
    return x * 3


@workflow
async def path_wf(x: int):
    result = await run_external(x=x)
    return result


@workflow
async def mixed_step_task_wf(x: int):
    ts = await step("get_time", lambda: 999)
    doubled = await double(x=x)
    config = await step("get_config", lambda: {"retry": 3})
    added = await add_one(x=doubled)
    return {"ts": ts, "doubled": doubled, "config": config, "added": added}


@workflow
async def par_child_wf():
    a, b = await asyncio.gather(double(x=3), add_one(x=7))
    return {"a": a, "b": b}


@workflow
async def det_wf(n: int):
    a = await double(x=n)
    b = await add_one(x=a)
    c = await double(x=b)
    return c


@workflow
async def par_args_wf(x: int):
    base = await double(x=x)
    a, b = await asyncio.gather(add_one(x=base), double(x=base))
    return {"a": a, "b": b}


@workflow
async def none_return_wf():
    await double(x=1)


@workflow
async def large_par_wf():
    results = await asyncio.gather(
        double(x=1), double(x=2), double(x=3), double(x=4), double(x=5)
    )
    return list(results)


@workflow
async def complex_mixed_wf():
    init = await extract_data(url="start")
    a, b = await asyncio.gather(double(x=1), double(x=2))
    mid = await load_data(data={"a": a, "b": b})
    c, d = await asyncio.gather(add_one(x=3), add_one(x=4))
    fin = await clean_data(data={"mid": mid, "c": c, "d": d})
    return fin


@workflow
async def pre_par_child_wf(x: int):
    base = await double(x=x)
    a, b = await asyncio.gather(add_one(x=base), double(x=base))
    return {"a": a, "b": b}


# --- Tests ---
# NOTE: Python SDK uses name-based keys (e.g. "double", "double_2")
# not index-based keys (e.g. "step_0", "step_1").

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

    def test_preserves_function_name(self):
        assert extract_data.__name__ == "extract_data"
        assert double.__name__ == "double"


class TestFirstInvocation:
    def test_dispatches_first_step(self):
        result = _run_workflow(simple_workflow, {}, {"url": "https://example.com"})
        assert result["type"] == "dispatch"
        assert result["mode"] == "sequential"
        assert len(result["steps"]) == 1
        assert result["steps"][0]["name"] == "extract_data"
        assert result["steps"][0]["script"] == "extract_data"
        assert result["steps"][0]["key"] == "extract_data"
        assert result["steps"][0]["args"] == {"url": "https://example.com"}

    def test_positional_args_converted_to_kwargs(self):
        """Positional args should be mapped to parameter names in dispatch."""
        @workflow
        async def pos_workflow():
            await extract_data("https://pos.example.com")

        result = _run_workflow(pos_workflow, {}, {})
        assert result["type"] == "dispatch"
        assert result["steps"][0]["args"] == {"url": "https://pos.example.com"}


class TestReplayWithCheckpoint:
    def test_second_invocation_dispatches_second_step(self):
        checkpoint = {
            "completed_steps": {
                "extract_data": {"data": [1, 2, 3]},
            }
        }
        result = _run_workflow(simple_workflow, checkpoint, {"url": "https://example.com"})
        assert result["type"] == "dispatch"
        assert result["mode"] == "sequential"
        assert result["steps"][0]["name"] == "load_data"
        assert result["steps"][0]["key"] == "load_data"

    def test_all_steps_complete(self):
        checkpoint = {
            "completed_steps": {
                "extract_data": {"data": [1, 2, 3]},
                "load_data": {"loaded": True},
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
                "extract_data": {"raw": "data"},
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
                "extract_data": {"raw": "data"},
                "clean_data": {"cleaned": True},
                "compute_stats": {"count": 42},
            }
        }
        result = _run_workflow(parallel_workflow, checkpoint, {"url": "https://example.com"})
        assert result["type"] == "complete"
        assert result["result"]["cleaned"] == {"cleaned": True}
        assert result["result"]["stats"] == {"count": 42}


class TestConditionalWorkflow:
    def test_condition_true(self):
        result = _run_workflow(conditional_workflow, {}, {"count": 200})
        assert result["type"] == "dispatch"
        assert result["steps"][0]["name"] == "send_alert"

    def test_condition_false(self):
        result = _run_workflow(conditional_workflow, {}, {"count": 50})
        assert result["type"] == "dispatch"
        assert result["steps"][0]["name"] == "load_data"


class TestStepInlineCheckpoint:
    def test_first_invocation_returns_inline_checkpoint(self):
        result = _run_workflow(step_workflow, {}, {"x": 7})
        assert result["type"] == "inline_checkpoint"
        assert result["key"] == "timestamp"
        assert result["result"] == 1234567890

    def test_step_cached_then_task_dispatches(self):
        checkpoint = {"completed_steps": {"timestamp": 1234567890}}
        result = _run_workflow(step_workflow, checkpoint, {"x": 7})
        assert result["type"] == "dispatch"
        assert result["mode"] == "sequential"
        assert result["steps"][0]["name"] == "double"
        assert result["steps"][0]["key"] == "double"

    def test_step_and_task_cached_then_second_step(self):
        checkpoint = {"completed_steps": {"timestamp": 1234567890, "double": 14}}
        result = _run_workflow(step_workflow, checkpoint, {"x": 7})
        assert result["type"] == "inline_checkpoint"
        assert result["key"] == "random_id"
        assert result["result"] == "abc-123"

    def test_all_complete(self):
        checkpoint = {"completed_steps": {"timestamp": 1234567890, "double": 14, "random_id": "abc-123"}}
        result = _run_workflow(step_workflow, checkpoint, {"x": 7})
        assert result["type"] == "complete"
        assert result["result"] == {"ts": 1234567890, "doubled": 14, "id": "abc-123"}


class TestUnawaitedTask:
    def test_unawaited_last_task_is_flushed(self):
        @workflow
        async def unawaited_workflow():
            await extract_data(url="x")
            load_data(data="y")

        checkpoint = {"completed_steps": {"extract_data": "raw"}}
        result = _run_workflow(unawaited_workflow, checkpoint, {})
        assert result["type"] == "dispatch"
        assert result["mode"] == "sequential"
        assert len(result["steps"]) == 1
        assert result["steps"][0]["name"] == "load_data"

    def test_unawaited_multiple_tasks_flushed_as_parallel(self):
        @workflow
        async def multi_unawaited_workflow():
            await extract_data(url="x")
            clean_data(data="y")
            compute_stats(data="y")

        checkpoint = {"completed_steps": {"extract_data": "raw"}}
        result = _run_workflow(multi_unawaited_workflow, checkpoint, {})
        assert result["type"] == "dispatch"
        assert result["mode"] == "parallel"
        assert len(result["steps"]) == 2
        assert result["steps"][0]["name"] == "clean_data"
        assert result["steps"][1]["name"] == "compute_stats"


class TestChildMode:
    def test_child_executes_matching_task(self):
        checkpoint = {"completed_steps": {"timestamp": 1234567890}, "_executing_key": "double"}
        result = _run_workflow(step_workflow, checkpoint, {"x": 7})
        assert result["type"] == "complete"
        assert result["result"] == 14

    def test_child_replays_cached_steps(self):
        checkpoint = {
            "completed_steps": {"extract_data": {"data": [1, 2, 3]}},
            "_executing_key": "load_data",
        }
        result = _run_workflow(simple_workflow, checkpoint, {"url": "https://example.com"})
        assert result["type"] == "complete"
        assert result["result"] is None


# =====================================================================
# EDGE CASE TESTS
# =====================================================================

class TestFullSequentialLifecycle:
    def test_replay_0_dispatches_step_0(self):
        result = _run_workflow(three_step_wf, {}, {"n": 5})
        assert result["type"] == "dispatch"
        assert result["steps"][0]["key"] == "double"
        assert result["steps"][0]["name"] == "double"
        assert result["steps"][0]["args"] == {"x": 5}

    def test_replay_1_dispatches_step_1_with_step_0_result(self):
        result = _run_workflow(three_step_wf, {"completed_steps": {"double": 10}}, {"n": 5})
        assert result["type"] == "dispatch"
        assert result["steps"][0]["key"] == "add_one"
        assert result["steps"][0]["name"] == "add_one"
        assert result["steps"][0]["args"] == {"x": 10}

    def test_replay_2_dispatches_step_2_with_step_1_result(self):
        result = _run_workflow(
            three_step_wf, {"completed_steps": {"double": 10, "add_one": 11}}, {"n": 5}
        )
        assert result["type"] == "dispatch"
        assert result["steps"][0]["key"] == "double_2"
        assert result["steps"][0]["name"] == "double"
        assert result["steps"][0]["args"] == {"x": 11}

    def test_replay_3_all_complete(self):
        result = _run_workflow(
            three_step_wf,
            {"completed_steps": {"double": 10, "add_one": 11, "double_2": 22}},
            {"n": 5},
        )
        assert result["type"] == "complete"
        assert result["result"] == {"doubled": 10, "incremented": 11, "final": 22}


class TestStepAfterParallelGroup:
    def test_dispatches_first_sequential(self):
        result = _run_workflow(seq_par_seq_wf, {}, {"url": "http://x"})
        assert result["steps"][0]["name"] == "extract_data"

    def test_dispatches_parallel_group(self):
        result = _run_workflow(
            seq_par_seq_wf, {"completed_steps": {"extract_data": "raw"}}, {"url": "http://x"}
        )
        assert result["mode"] == "parallel"
        assert len(result["steps"]) == 2

    def test_dispatches_final_step_after_parallel(self):
        result = _run_workflow(
            seq_par_seq_wf,
            {"completed_steps": {"extract_data": "raw", "clean_data": "cleaned", "compute_stats": {"count": 5}}},
            {"url": "http://x"},
        )
        assert result["mode"] == "sequential"
        assert result["steps"][0]["name"] == "load_data"
        assert result["steps"][0]["key"] == "load_data"

    def test_completes_when_final_step_done(self):
        result = _run_workflow(
            seq_par_seq_wf,
            {"completed_steps": {"extract_data": "raw", "clean_data": "cleaned", "compute_stats": {"count": 5}, "load_data": "final"}},
            {"url": "http://x"},
        )
        assert result["type"] == "complete"
        assert result["result"] == "final"


class TestParallelAfterParallel:
    def test_dispatches_first_parallel(self):
        result = _run_workflow(double_parallel_wf, {}, {})
        assert result["mode"] == "parallel"
        assert len(result["steps"]) == 2
        assert result["steps"][0]["key"] == "double"
        assert result["steps"][1]["key"] == "double_2"

    def test_dispatches_second_parallel(self):
        result = _run_workflow(
            double_parallel_wf, {"completed_steps": {"double": 2, "double_2": 4}}, {}
        )
        assert result["mode"] == "parallel"
        assert len(result["steps"]) == 2
        assert result["steps"][0]["name"] == "add_one"
        assert result["steps"][0]["args"] == {"x": 2}
        assert result["steps"][1]["args"] == {"x": 4}

    def test_completes_all_done(self):
        result = _run_workflow(
            double_parallel_wf,
            {"completed_steps": {"double": 2, "double_2": 4, "add_one": 3, "add_one_2": 5}},
            {},
        )
        assert result["type"] == "complete"
        assert result["result"] == {"a": 2, "b": 4, "c": 3, "d": 5}


class TestConditionalBasedOnStepResult:
    def test_condition_true_path(self):
        result = _run_workflow(cond_on_result_wf, {"completed_steps": {"double": 10}}, {})
        assert result["steps"][0]["name"] == "send_alert"
        assert result["steps"][0]["key"] == "send_alert"

    def test_condition_false_path(self):
        result = _run_workflow(cond_on_result_wf, {"completed_steps": {"double": 4}}, {})
        assert result["steps"][0]["name"] == "load_data"
        assert result["steps"][0]["key"] == "load_data"

    def test_condition_true_step_after_alert(self):
        result = _run_workflow(
            cond_on_result_wf, {"completed_steps": {"double": 10, "send_alert": "alerted"}}, {}
        )
        assert result["steps"][0]["name"] == "load_data"
        assert result["steps"][0]["key"] == "load_data"


class TestEmptyWorkflow:
    def test_completes_immediately(self):
        result = _run_workflow(empty_wf, {}, {})
        assert result["type"] == "complete"
        assert result["result"] == {"status": "empty"}


class TestSingleTaskWorkflow:
    def test_dispatches_single_step(self):
        result = _run_workflow(single_wf, {}, {"x": 7})
        assert result["type"] == "dispatch"
        assert len(result["steps"]) == 1
        assert result["steps"][0]["name"] == "double"

    def test_completes_with_result(self):
        result = _run_workflow(single_wf, {"completed_steps": {"double": 14}}, {"x": 7})
        assert result["type"] == "complete"
        assert result["result"] == 14


class TestTaskWithNoArgs:
    def test_dispatches_with_empty_args(self):
        result = _run_workflow(no_arg_wf, {}, {})
        assert result["type"] == "dispatch"
        assert result["steps"][0]["args"] == {}


class TestManySteps:
    def test_first_dispatches_step_0(self):
        result = _run_workflow(many_steps_wf, {}, {"n": 0})
        assert result["steps"][0]["key"] == "add_one"

    def test_with_5_complete_dispatches_step_5(self):
        # add_one, add_one_2, add_one_3, add_one_4, add_one_5
        completed = {}
        for i in range(5):
            key = "add_one" if i == 0 else f"add_one_{i + 1}"
            completed[key] = i + 1
        result = _run_workflow(many_steps_wf, {"completed_steps": completed}, {"n": 0})
        assert result["steps"][0]["key"] == "add_one_6"
        assert result["steps"][0]["args"] == {"x": 5}

    def test_all_10_complete(self):
        completed = {}
        for i in range(10):
            key = "add_one" if i == 0 else f"add_one_{i + 1}"
            completed[key] = i + 1
        result = _run_workflow(many_steps_wf, {"completed_steps": completed}, {"n": 0})
        assert result["type"] == "complete"
        assert result["result"] == 10


class TestFalsyValues:
    def test_zero_preserved(self):
        result = _run_workflow(falsy_wf, {"completed_steps": {"double": 0}}, {})
        assert result["type"] == "dispatch"
        assert result["steps"][0]["name"] == "load_data"
        assert result["steps"][0]["args"] == {"data": 0}

    def test_none_preserved(self):
        result = _run_workflow(falsy_wf, {"completed_steps": {"double": 0, "load_data": None}}, {})
        assert result["type"] == "dispatch"
        assert result["steps"][0]["name"] == "extract_data"

    def test_all_falsy_complete(self):
        result = _run_workflow(
            falsy_wf, {"completed_steps": {"double": 0, "load_data": None, "extract_data": ""}}, {}
        )
        assert result["type"] == "complete"
        assert result["result"] == {"a": 0, "b": None, "c": ""}

    def test_false_preserved(self):
        @workflow
        async def flag_wf():
            val = await load_data(data="check")
            if val:
                await send_alert(msg="truthy")
            return {"val": val}

        result = _run_workflow(flag_wf, {"completed_steps": {"load_data": False}}, {})
        assert result["type"] == "complete"
        assert result["result"] == {"val": False}


class TestTaskWithExplicitPath:
    def test_uses_path_as_script(self):
        result = _run_workflow(path_wf, {}, {"x": 42})
        assert result["type"] == "dispatch"
        assert result["steps"][0]["name"] == "run_external"
        assert result["steps"][0]["script"] == "f/external_script"
        assert result["steps"][0]["args"] == {"x": 42}


class TestMixedStepAndTask:
    def test_step_0_inline(self):
        result = _run_workflow(mixed_step_task_wf, {}, {"x": 5})
        assert result["type"] == "inline_checkpoint"
        assert result["key"] == "get_time"
        assert result["result"] == 999

    def test_step_1_task_dispatch(self):
        result = _run_workflow(
            mixed_step_task_wf, {"completed_steps": {"get_time": 999}}, {"x": 5}
        )
        assert result["type"] == "dispatch"
        assert result["steps"][0]["name"] == "double"
        assert result["steps"][0]["key"] == "double"

    def test_step_2_inline(self):
        result = _run_workflow(
            mixed_step_task_wf,
            {"completed_steps": {"get_time": 999, "double": 10}},
            {"x": 5},
        )
        assert result["type"] == "inline_checkpoint"
        assert result["key"] == "get_config"
        assert result["result"] == {"retry": 3}

    def test_step_3_task_dispatch(self):
        result = _run_workflow(
            mixed_step_task_wf,
            {"completed_steps": {"get_time": 999, "double": 10, "get_config": {"retry": 3}}},
            {"x": 5},
        )
        assert result["type"] == "dispatch"
        assert result["steps"][0]["name"] == "add_one"
        assert result["steps"][0]["key"] == "add_one"

    def test_all_complete(self):
        result = _run_workflow(
            mixed_step_task_wf,
            {"completed_steps": {"get_time": 999, "double": 10, "get_config": {"retry": 3}, "add_one": 11}},
            {"x": 5},
        )
        assert result["type"] == "complete"
        assert result["result"] == {"ts": 999, "doubled": 10, "config": {"retry": 3}, "added": 11}


class TestChildModeParallel:
    def test_child_executes_first_parallel_step(self):
        result = _run_workflow(
            par_child_wf, {"completed_steps": {}, "_executing_key": "double"}, {}
        )
        assert result["type"] == "complete"
        assert result["result"] == 6

    def test_child_executes_second_parallel_step(self):
        result = _run_workflow(
            par_child_wf, {"completed_steps": {}, "_executing_key": "add_one"}, {}
        )
        assert result["type"] == "complete"
        assert result["result"] == 8


class TestKeyDeterminism:
    def test_keys_consistent_across_replays(self):
        r1 = _run_workflow(det_wf, {}, {"n": 3})
        assert r1["steps"][0]["key"] == "double"
        assert r1["steps"][0]["name"] == "double"

        r2 = _run_workflow(det_wf, {"completed_steps": {"double": 6}}, {"n": 3})
        assert r2["steps"][0]["key"] == "add_one"
        assert r2["steps"][0]["name"] == "add_one"

        r3 = _run_workflow(det_wf, {"completed_steps": {"double": 6, "add_one": 7}}, {"n": 3})
        assert r3["steps"][0]["key"] == "double_2"
        assert r3["steps"][0]["name"] == "double"


class TestParallelArgsFromCachedResult:
    def test_parallel_steps_receive_cached_args(self):
        result = _run_workflow(par_args_wf, {"completed_steps": {"double": 20}}, {"x": 10})
        assert result["mode"] == "parallel"
        assert result["steps"][0]["args"] == {"x": 20}
        assert result["steps"][1]["args"] == {"x": 20}


class TestWorkflowReturningNone:
    def test_none_return_captured(self):
        result = _run_workflow(none_return_wf, {"completed_steps": {"double": 2}}, {})
        assert result["type"] == "complete"
        assert result["result"] is None


class TestLargeParallelGroup:
    def test_dispatches_5_parallel(self):
        result = _run_workflow(large_par_wf, {}, {})
        assert result["mode"] == "parallel"
        assert len(result["steps"]) == 5
        keys = [result["steps"][i]["key"] for i in range(5)]
        assert keys == ["double", "double_2", "double_3", "double_4", "double_5"]
        for i in range(5):
            assert result["steps"][i]["args"] == {"x": i + 1}


class TestComplexMixedWorkflow:
    def test_replay_0_extract(self):
        r = _run_workflow(complex_mixed_wf, {}, {})
        assert r["steps"][0]["name"] == "extract_data"

    def test_replay_1_parallel(self):
        r = _run_workflow(complex_mixed_wf, {"completed_steps": {"extract_data": "init"}}, {})
        assert r["mode"] == "parallel"
        assert len(r["steps"]) == 2

    def test_replay_2_load(self):
        r = _run_workflow(
            complex_mixed_wf,
            {"completed_steps": {"extract_data": "init", "double": 2, "double_2": 4}},
            {},
        )
        assert r["mode"] == "sequential"
        assert r["steps"][0]["name"] == "load_data"
        assert r["steps"][0]["key"] == "load_data"

    def test_replay_3_second_parallel(self):
        r = _run_workflow(
            complex_mixed_wf,
            {"completed_steps": {"extract_data": "init", "double": 2, "double_2": 4, "load_data": "mid"}},
            {},
        )
        assert r["mode"] == "parallel"
        assert len(r["steps"]) == 2
        assert r["steps"][0]["name"] == "add_one"

    def test_replay_4_clean(self):
        r = _run_workflow(
            complex_mixed_wf,
            {"completed_steps": {
                "extract_data": "init", "double": 2, "double_2": 4,
                "load_data": "mid", "add_one": 4, "add_one_2": 5,
            }},
            {},
        )
        assert r["mode"] == "sequential"
        assert r["steps"][0]["name"] == "clean_data"
        assert r["steps"][0]["key"] == "clean_data"

    def test_replay_5_all_complete(self):
        r = _run_workflow(
            complex_mixed_wf,
            {"completed_steps": {
                "extract_data": "init", "double": 2, "double_2": 4,
                "load_data": "mid", "add_one": 4, "add_one_2": 5, "clean_data": "final",
            }},
            {},
        )
        assert r["type"] == "complete"
        assert r["result"] == "final"


class TestChildModeWithCachedStepsBeforeParallel:
    def test_child_executes_second_parallel_with_cached_base(self):
        result = _run_workflow(
            pre_par_child_wf,
            {"completed_steps": {"double": 10}, "_executing_key": "double_2"},
            {"x": 5},
        )
        assert result["type"] == "complete"
        assert result["result"] == 20

    def test_child_executes_first_parallel_with_cached_base(self):
        result = _run_workflow(
            pre_par_child_wf,
            {"completed_steps": {"double": 10}, "_executing_key": "add_one"},
            {"x": 5},
        )
        assert result["type"] == "complete"
        assert result["result"] == 11


# =====================================================================
# ERROR PROPAGATION TESTS
# =====================================================================


class TestErrorPropagation:
    def test_task_error_is_raised_on_replay(self):
        @workflow
        async def wf(x: int):
            return await double(x=x)

        with pytest.raises(TaskError, match="double"):
            _run_workflow(
                wf,
                {
                    "completed_steps": {
                        "double": {
                            "__wmill_error": True,
                            "message": "Task 'double' failed",
                            "result": {"message": "boom"},
                        }
                    }
                },
                {"x": 5},
            )

    def test_error_catchable_with_try_except(self):
        @workflow
        async def wf(x: int):
            try:
                result = await double(x=x)
                return {"success": True, "result": result}
            except Exception as e:
                return {"success": False, "error": str(e)}

        r = _run_workflow(
            wf,
            {
                "completed_steps": {
                    "double": {
                        "__wmill_error": True,
                        "message": "Task 'double' failed",
                        "result": {},
                    }
                }
            },
            {"x": 5},
        )
        assert r["type"] == "complete"
        assert r["result"]["success"] is False
        assert "double" in r["result"]["error"]

    def test_retry_pattern_with_try_except_loop(self):
        @workflow
        async def wf(x: int):
            for i in range(3):
                try:
                    result = await double(x=x)
                    return {"result": result, "attempts": i + 1}
                except Exception:
                    if i == 2:
                        raise

        # First double fails, second succeeds
        r = _run_workflow(
            wf,
            {
                "completed_steps": {
                    "double": {"__wmill_error": True, "message": "temporary", "result": {}},
                    "double_2": 10,
                }
            },
            {"x": 5},
        )
        assert r["type"] == "complete"
        assert r["result"]["result"] == 10
        assert r["result"]["attempts"] == 2

    def test_non_error_object_with_error_false(self):
        @workflow
        async def wf():
            val = await double(x=5)
            return val

        r = _run_workflow(
            wf,
            {"completed_steps": {"double": {"__wmill_error": False, "data": "ok"}}},
            {},
        )
        assert r["type"] == "complete"
        assert r["result"] == {"__wmill_error": False, "data": "ok"}

    def test_inline_step_error(self):
        @workflow
        async def wf():
            try:
                val = await step("risky", lambda: 42)
                return {"val": val}
            except Exception as e:
                return {"caught": str(e)}

        r = _run_workflow(
            wf,
            {"completed_steps": {"risky": {"__wmill_error": True, "message": "step failed", "result": {}}}},
            {},
        )
        assert r["type"] == "complete"
        assert "step failed" in r["result"]["caught"]


# =====================================================================
# TASK OPTIONS TESTS
# =====================================================================


class TestTaskOptions:
    def test_options_forwarded_in_dispatch(self):
        @task(timeout=600, tag="gpu", cache_ttl=3600, priority=10)
        async def heavy(x: int):
            return x

        @workflow
        async def wf(x: int):
            return await heavy(x=x)

        r = _run_workflow(wf, {}, {"x": 42})
        assert r["type"] == "dispatch"
        step_info = r["steps"][0]
        assert step_info["timeout"] == 600
        assert step_info["tag"] == "gpu"
        assert step_info["cache_ttl"] == 3600
        assert step_info["priority"] == 10

    def test_task_without_options_has_no_extra_fields(self):
        @task
        async def simple(x: int):
            return x

        @workflow
        async def wf(x: int):
            return await simple(x=x)

        r = _run_workflow(wf, {}, {"x": 1})
        step_info = r["steps"][0]
        assert "timeout" not in step_info
        assert "tag" not in step_info

    def test_concurrency_options_forwarded(self):
        @task(concurrency_limit=5, concurrency_key="my-key", concurrency_time_window_s=60)
        async def limited(x: int):
            return x

        @workflow
        async def wf(x: int):
            return await limited(x=x)

        r = _run_workflow(wf, {}, {"x": 1})
        step_info = r["steps"][0]
        assert step_info["concurrent_limit"] == 5
        assert step_info["concurrency_key"] == "my-key"
        assert step_info["concurrency_time_window_s"] == 60


# =====================================================================
# SLEEP TESTS
# =====================================================================


class TestSleep:
    def test_sleep_returns_sleep_output(self):
        @workflow
        async def wf():
            await double(x=1)
            await sleep(60)
            await add_one(x=2)
            return "done"

        r = _run_workflow(
            wf,
            {"completed_steps": {"double": 2}},
            {},
        )
        assert r["type"] == "sleep"
        assert r["key"] == "sleep"
        assert r["seconds"] == 60

    def test_sleep_completes_on_replay(self):
        @workflow
        async def wf():
            await double(x=1)
            await sleep(60)
            await add_one(x=2)
            return "done"

        r = _run_workflow(
            wf,
            {"completed_steps": {"double": 2, "sleep": True}},
            {},
        )
        assert r["type"] == "dispatch"
        assert r["steps"][0]["name"] == "add_one"
        assert r["steps"][0]["key"] == "add_one"

    def test_all_steps_with_sleep_complete(self):
        @workflow
        async def wf():
            await double(x=1)
            await sleep(60)
            await add_one(x=2)
            return "done"

        r = _run_workflow(
            wf,
            {"completed_steps": {"double": 2, "sleep": True, "add_one": 3}},
            {},
        )
        assert r["type"] == "complete"
        assert r["result"] == "done"

    def test_sleep_enforces_minimum(self):
        @workflow
        async def wf():
            await sleep(0)
            return "done"

        r = _run_workflow(wf, {}, {})
        assert r["seconds"] == 1


# =====================================================================
# PARALLEL UTILITY TESTS
# =====================================================================


class TestParallel:
    def test_dispatches_all_items(self):
        @workflow
        async def wf():
            results = await parallel([1, 2, 3], double)
            return results

        r = _run_workflow(wf, {}, {})
        assert r["type"] == "dispatch"
        assert r["mode"] == "parallel"
        assert len(r["steps"]) == 3

    def test_completes_with_all_results(self):
        @workflow
        async def wf():
            results = await parallel([1, 2, 3], double)
            return results

        r = _run_workflow(
            wf,
            {"completed_steps": {"double": 2, "double_2": 4, "double_3": 6}},
            {},
        )
        assert r["type"] == "complete"
        assert r["result"] == [2, 4, 6]

    def test_batched_dispatches_first_batch(self):
        @workflow
        async def wf():
            results = await parallel([1, 2, 3, 4, 5], double, concurrency=2)
            return results

        r = _run_workflow(wf, {}, {})
        assert r["type"] == "dispatch"
        assert r["mode"] == "parallel"
        assert len(r["steps"]) == 2

    def test_batched_dispatches_second_batch(self):
        @workflow
        async def wf():
            results = await parallel([1, 2, 3, 4, 5], double, concurrency=2)
            return results

        r = _run_workflow(
            wf,
            {"completed_steps": {"double": 2, "double_2": 4}},
            {},
        )
        assert r["type"] == "dispatch"
        assert len(r["steps"]) == 2

    def test_batched_completes_with_all_results(self):
        @workflow
        async def wf():
            results = await parallel([1, 2, 3, 4, 5], double, concurrency=2)
            return results

        r = _run_workflow(
            wf,
            {"completed_steps": {"double": 2, "double_2": 4, "double_3": 6, "double_4": 8, "double_5": 10}},
            {},
        )
        assert r["type"] == "complete"
        assert r["result"] == [2, 4, 6, 8, 10]

    def test_empty_items_returns_empty(self):
        @workflow
        async def wf():
            results = await parallel([], double)
            return results

        r = _run_workflow(wf, {}, {})
        assert r["type"] == "complete"
        assert r["result"] == []
