from typing import Any, Union, Dict
from typing import Generic, TypeVar, TypeAlias

import os

from time import sleep
from windmill_api.models.whoami_response_200 import WhoamiResponse200

from windmill_api.client import AuthenticatedClient

from enum import Enum

from windmill_api.types import Unset

S = TypeVar("S")


class Resource(Generic[S]):
    pass


postgresql = TypeAlias
mysql = TypeAlias
bigquery = TypeAlias


VAR_RESOURCE_PREFIX = "$var:"
RES_RESOURCE_PREFIX = "$res:"


class JobStatus(Enum):
    WAITING = 1
    RUNNING = 2
    COMPLETED = 3


_client: "AuthenticatedClient | None" = None


def create_client(
    base_url: "str | None" = None, token: "str | None" = None
) -> AuthenticatedClient:
    env_base_url = os.environ.get("BASE_INTERNAL_URL")

    if env_base_url is not None:
        env_base_url = env_base_url + "/api"

    base_url_: str = base_url or env_base_url or "http://localhost:8000/api"
    token_: str = token or os.environ.get("WM_TOKEN") or ""
    global _client
    if _client is None:
        _client = AuthenticatedClient(
            base_url=base_url_, token=token_, timeout=30, verify_ssl=False
        )
    return _client


def get_workspace() -> str:
    from_env = os.environ.get("WM_WORKSPACE")
    if from_env is None:
        raise Exception("Workspace not passed as WM_WORKSPACE")
    return from_env


def get_version() -> str:
    """
    Returns the current version of the backend
    """
    from windmill_api.api.settings import backend_version

    return backend_version.sync_detailed(client=create_client()).content.decode(
        "us-ascii"
    )


def run_script_async(
    hash: str,
    args: Dict[str, Any] = {},
    scheduled_in_secs: Union[None, int] = None,
) -> str:
    """
    Launch the run of a script and return immediately its job id
    """
    from windmill_api.api.job import run_script_by_hash

    from windmill_api.models.run_script_by_hash_json_body import RunScriptByHashJsonBody

    return run_script_by_hash.sync_detailed(
        client=create_client(),
        workspace=get_workspace(),
        hash_=hash,
        json_body=RunScriptByHashJsonBody.from_dict(args),
        scheduled_in_secs=scheduled_in_secs,
        parent_job=os.environ.get("DT_JOB_ID"),
    ).content.decode("us-ascii")


def run_script_sync(
    hash: str, args: Dict[str, Any] = {}, verbose: bool = False
) -> Dict[str, Any]:
    """
    Run a script, wait for it to complete and return the result of the launched script
    """
    job_id = run_script_async(hash, args, None)
    nb_iter = 0
    while get_job_status(job_id) != JobStatus.COMPLETED:
        if verbose:
            print(f"Waiting for {job_id} to complete...")
        if nb_iter < 10:
            sleep(2.0)
        else:
            sleep(5.0)
        nb_iter += 1
    return get_result(job_id)


def get_job_status(job_id: str) -> JobStatus:
    """
    Returns the status of a queued or completed job
    """
    from windmill_api.models.get_job_response_200_type import GetJobResponse200Type
    from windmill_api.api.job import get_job

    res = get_job.sync_detailed(
        client=create_client(), workspace=get_workspace(), id=job_id
    ).parsed
    if not res:
        raise Exception(f"Job {job_id} not found")
    elif not res.type:
        raise Exception(f"Unexpected type not found for job {job_id}")
    elif res.type == GetJobResponse200Type.COMPLETEDJOB:
        return JobStatus.COMPLETED
    else:
        if not "running" in res.additional_properties:
            raise Exception(f"Unexpected running not found for completed job {job_id}")
        elif bool(res.additional_properties["running"]):
            return JobStatus.RUNNING
        else:
            return JobStatus.WAITING


def get_result(job_id: str) -> Dict[str, Any]:
    """
    Returns the result of a completed job
    """
    from windmill_api.api.job import get_completed_job

    res = get_completed_job.sync_detailed(
        client=create_client(), workspace=get_workspace(), id=job_id
    ).parsed
    if not res:
        raise Exception(f"Job {job_id} not found")
    if not res.result:
        raise Exception(f"Unexpected result not found for completed job {job_id}")
    else:
        return res.result.to_dict()  # type: ignore


def get_resource(path: str | None = None, none_if_undefined: bool = False) -> Any:
    """
    Returns the resource at a given path
    """
    from windmill_api.api.resource import get_resource as get_resource_api

    path = path or get_state_path()
    parsed = get_resource_api.sync_detailed(
        workspace=get_workspace(), path=path, client=create_client()
    ).parsed
    if parsed is None:
        if none_if_undefined:
            return None
        else:
            raise Exception(
                f"Resource at path {path} does not exist or you do not have read permissions on it"
            )

    if isinstance(parsed.value, Unset):
        return None

    raw = parsed.value
    return _transform_leaf(raw)


def whoami() -> WhoamiResponse200 | None:
    """
    Returns the current user
    """
    from windmill_api.api.user import whoami

    return whoami.sync(client=create_client(), workspace=get_workspace())


def get_state() -> Any:
    """
    Get the state
    """
    return get_resource(None, True)


def set_resource(
    value: Any, path: str | None = None, resource_type: str = "state"
) -> None:
    """
    Set the resource at a given path as a string, creating it if it does not exist
    """
    from windmill_api.models.create_resource_json_body import CreateResourceJsonBody
    from windmill_api.models.update_resource_value_json_body import (
        UpdateResourceValueJsonBody,
    )
    from windmill_api.api.resource import (
        exists_resource,
        update_resource_value,
        create_resource,
    )

    path = path or get_state_path()
    workspace = get_workspace()
    client = create_client()
    if not exists_resource.sync_detailed(
        workspace=workspace, path=path, client=client
    ).parsed:
        create_resource.sync_detailed(
            workspace=workspace,
            client=client,
            json_body=CreateResourceJsonBody(
                path=path, value=value, resource_type=resource_type
            ),
        )
    else:
        update_resource_value.sync_detailed(
            workspace=get_workspace(),
            client=client,
            path=path,
            json_body=UpdateResourceValueJsonBody(value=value),
        )


def set_state(value: Any) -> None:
    """
    Set the state
    """
    set_resource(value, None)


def set_shared_state_pickle(value: Any, path: str = "state.pickle") -> None:
    """
    Set the state in the shared folder using pickle
    """
    import pickle

    with open(f"/shared/{path}", "wb") as handle:
        pickle.dump(value, handle, protocol=pickle.HIGHEST_PROTOCOL)


def get_shared_state_pickle(path: str = "state.pickle") -> Any:
    """
    Get the state in the shared folder using pickle
    """
    import pickle

    with open(f"/shared/{path}", "rb") as handle:
        return pickle.load(handle)


def set_shared_state(value: Any, path: str = "state.json") -> None:
    """
    Set the state in the shared folder using pickle
    """
    import json

    with open(f"/shared/{path}", "w", encoding="utf-8") as f:
        json.dump(value, f, ensure_ascii=False, indent=4)


def get_shared_state(path: str = "state.json") -> None:
    """
    Set the state in the shared folder using pickle
    """
    import json

    with open(f"/shared/{path}", "r", encoding="utf-8") as f:
        return json.load(f)


def get_variable(path: str) -> str:
    """
    Returns the variable at a given path as a string
    """
    from windmill_api.api.variable import get_variable as get_variable_api

    res = get_variable_api.sync_detailed(
        workspace=get_workspace(), path=path, client=create_client()
    ).parsed
    if res is None:
        raise Exception(
            f"Variable at path {path} does not exist or you do not have read permissions on it"
        )

    return res.value  # type: ignore


def set_variable(path: str, value: str) -> None:
    """
    Set the variable at a given path as a string, creating it if it does not exist
    """
    from windmill_api.api.variable import (
        exists_variable,
        update_variable,
        create_variable,
    )

    from windmill_api.models.update_variable_json_body import UpdateVariableJsonBody
    from windmill_api.models.create_variable_json_body import CreateVariableJsonBody

    workspace = get_workspace()
    client = create_client()
    if not exists_variable.sync_detailed(
        workspace=workspace, path=path, client=client
    ).parsed:
        create_variable.sync_detailed(
            workspace=workspace,
            client=client,
            json_body=CreateVariableJsonBody(
                path=path, value=value, is_secret=False, description=""
            ),
        )
    else:
        update_variable.sync_detailed(
            workspace=get_workspace(),
            path=path,
            client=client,
            json_body=UpdateVariableJsonBody(value=value),
        )


def get_state_path() -> str:
    state_path = os.environ.get("WM_STATE_PATH")
    if state_path is None:
        raise Exception("State path not found")
    return state_path


def get_resume_urls(approver: str | None = None) -> Dict:
    from windmill_api.api.job import get_resume_urls as get_resume_urls_api

    workspace = get_workspace()
    client = create_client()
    job_id = os.environ.get("WM_JOB_ID") or "NO_ID"
    import random

    nonce = random.randint(0, 1000000000)
    res = get_resume_urls_api.sync_detailed(
        workspace, job_id, nonce, client=client, approver=approver
    )
    if res.parsed is not None:
        return res.parsed.to_dict()
    else:
        raise Exception("Failed to get resume urls")


def _transform_leaves(d: Dict[str, Any]) -> Dict[str, Any]:
    return {k: _transform_leaf(v) for k, v in d.items()}


def _transform_leaf(v: Any) -> Any:
    if isinstance(v, dict):
        return _transform_leaves(v)  # type: ignore
    elif isinstance(v, str):
        if v.startswith(VAR_RESOURCE_PREFIX):
            var_name = v[len(VAR_RESOURCE_PREFIX) :]
            return get_variable(var_name)
        if v.startswith(RES_RESOURCE_PREFIX):
            res_name = v[len(RES_RESOURCE_PREFIX) :]
            return get_resource(res_name)
        else:
            return v
    else:
        return v
