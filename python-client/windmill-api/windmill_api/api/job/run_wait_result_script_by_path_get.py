from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import Union
from typing import Optional
from ...types import UNSET, Unset



def _get_kwargs(
    workspace: str,
    path: str,
    *,
    parent_job: Union[Unset, None, str] = UNSET,
    tag: Union[Unset, None, str] = UNSET,
    job_id: Union[Unset, None, str] = UNSET,
    include_header: Union[Unset, None, str] = UNSET,
    queue_limit: Union[Unset, None, str] = UNSET,
    payload: Union[Unset, None, str] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["parent_job"] = parent_job


    params["tag"] = tag


    params["job_id"] = job_id


    params["include_header"] = include_header


    params["queue_limit"] = queue_limit


    params["payload"] = payload



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    

    

    return {
        "method": "get",
        "url": "/w/{workspace}/jobs/run_wait_result/p/{path}".format(workspace=workspace,path=path,),
        "params": params,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[Any]:
    if response.status_code == HTTPStatus.OK:
        return None
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[Any]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    workspace: str,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],
    parent_job: Union[Unset, None, str] = UNSET,
    tag: Union[Unset, None, str] = UNSET,
    job_id: Union[Unset, None, str] = UNSET,
    include_header: Union[Unset, None, str] = UNSET,
    queue_limit: Union[Unset, None, str] = UNSET,
    payload: Union[Unset, None, str] = UNSET,

) -> Response[Any]:
    """ run script by path with get

    Args:
        workspace (str):
        path (str):
        parent_job (Union[Unset, None, str]):
        tag (Union[Unset, None, str]):
        job_id (Union[Unset, None, str]):
        include_header (Union[Unset, None, str]):
        queue_limit (Union[Unset, None, str]):
        payload (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
path=path,
parent_job=parent_job,
tag=tag,
job_id=job_id,
include_header=include_header,
queue_limit=queue_limit,
payload=payload,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


async def asyncio_detailed(
    workspace: str,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],
    parent_job: Union[Unset, None, str] = UNSET,
    tag: Union[Unset, None, str] = UNSET,
    job_id: Union[Unset, None, str] = UNSET,
    include_header: Union[Unset, None, str] = UNSET,
    queue_limit: Union[Unset, None, str] = UNSET,
    payload: Union[Unset, None, str] = UNSET,

) -> Response[Any]:
    """ run script by path with get

    Args:
        workspace (str):
        path (str):
        parent_job (Union[Unset, None, str]):
        tag (Union[Unset, None, str]):
        job_id (Union[Unset, None, str]):
        include_header (Union[Unset, None, str]):
        queue_limit (Union[Unset, None, str]):
        payload (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
path=path,
parent_job=parent_job,
tag=tag,
job_id=job_id,
include_header=include_header,
queue_limit=queue_limit,
payload=payload,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

