import datetime
from typing import Any, Dict, Union

import httpx

from ...client import Client
from ...models.run_wait_result_script_by_path_json_body import RunWaitResultScriptByPathJsonBody
from ...types import UNSET, Response, Unset


def _get_kwargs(
    workspace: str,
    path: str,
    *,
    client: Client,
    json_body: RunWaitResultScriptByPathJsonBody,
    scheduled_for: Union[Unset, None, datetime.datetime] = UNSET,
    scheduled_in_secs: Union[Unset, None, float] = UNSET,
    parent_job: Union[Unset, None, str] = UNSET,
) -> Dict[str, Any]:
    url = "{}/w/{workspace}/jobs/run_wait_result/p/{path}".format(client.base_url, workspace=workspace, path=path)

    headers: Dict[str, str] = client.get_headers()
    cookies: Dict[str, Any] = client.get_cookies()

    params: Dict[str, Any] = {}
    json_scheduled_for: Union[Unset, None, str] = UNSET
    if not isinstance(scheduled_for, Unset):
        json_scheduled_for = scheduled_for.isoformat() if scheduled_for else None

    params["scheduled_for"] = json_scheduled_for

    params["scheduled_in_secs"] = scheduled_in_secs

    params["parent_job"] = parent_job

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    json_json_body = json_body.to_dict()

    return {
        "method": "post",
        "url": url,
        "headers": headers,
        "cookies": cookies,
        "timeout": client.get_timeout(),
        "json": json_json_body,
        "params": params,
    }


def _build_response(*, response: httpx.Response) -> Response[Any]:
    return Response(
        status_code=response.status_code,
        content=response.content,
        headers=response.headers,
        parsed=None,
    )


def sync_detailed(
    workspace: str,
    path: str,
    *,
    client: Client,
    json_body: RunWaitResultScriptByPathJsonBody,
    scheduled_for: Union[Unset, None, datetime.datetime] = UNSET,
    scheduled_in_secs: Union[Unset, None, float] = UNSET,
    parent_job: Union[Unset, None, str] = UNSET,
) -> Response[Any]:
    """run script by path

    Args:
        workspace (str):
        path (str):
        scheduled_for (Union[Unset, None, datetime.datetime]):
        scheduled_in_secs (Union[Unset, None, float]):
        parent_job (Union[Unset, None, str]):
        json_body (RunWaitResultScriptByPathJsonBody):

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs(
        workspace=workspace,
        path=path,
        client=client,
        json_body=json_body,
        scheduled_for=scheduled_for,
        scheduled_in_secs=scheduled_in_secs,
        parent_job=parent_job,
    )

    response = httpx.request(
        verify=client.verify_ssl,
        **kwargs,
    )

    return _build_response(response=response)


async def asyncio_detailed(
    workspace: str,
    path: str,
    *,
    client: Client,
    json_body: RunWaitResultScriptByPathJsonBody,
    scheduled_for: Union[Unset, None, datetime.datetime] = UNSET,
    scheduled_in_secs: Union[Unset, None, float] = UNSET,
    parent_job: Union[Unset, None, str] = UNSET,
) -> Response[Any]:
    """run script by path

    Args:
        workspace (str):
        path (str):
        scheduled_for (Union[Unset, None, datetime.datetime]):
        scheduled_in_secs (Union[Unset, None, float]):
        parent_job (Union[Unset, None, str]):
        json_body (RunWaitResultScriptByPathJsonBody):

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs(
        workspace=workspace,
        path=path,
        client=client,
        json_body=json_body,
        scheduled_for=scheduled_for,
        scheduled_in_secs=scheduled_in_secs,
        parent_job=parent_job,
    )

    async with httpx.AsyncClient(verify=client.verify_ssl) as _client:
        response = await _client.request(**kwargs)

    return _build_response(response=response)
