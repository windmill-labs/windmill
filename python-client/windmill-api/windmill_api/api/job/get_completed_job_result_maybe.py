from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import Union
from typing import cast
from typing import Dict
from ...models.get_completed_job_result_maybe_response_200 import GetCompletedJobResultMaybeResponse200
from typing import Optional
from ...types import UNSET, Unset



def _get_kwargs(
    workspace: str,
    id: str,
    *,
    get_started: Union[Unset, None, bool] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["get_started"] = get_started



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    

    

    return {
        "method": "get",
        "url": "/w/{workspace}/jobs_u/completed/get_result_maybe/{id}".format(workspace=workspace,id=id,),
        "params": params,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[GetCompletedJobResultMaybeResponse200]:
    if response.status_code == HTTPStatus.OK:
        response_200 = GetCompletedJobResultMaybeResponse200.from_dict(response.json())



        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[GetCompletedJobResultMaybeResponse200]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    workspace: str,
    id: str,
    *,
    client: Union[AuthenticatedClient, Client],
    get_started: Union[Unset, None, bool] = UNSET,

) -> Response[GetCompletedJobResultMaybeResponse200]:
    """ get completed job result if job is completed

    Args:
        workspace (str):
        id (str):
        get_started (Union[Unset, None, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetCompletedJobResultMaybeResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
id=id,
get_started=get_started,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    workspace: str,
    id: str,
    *,
    client: Union[AuthenticatedClient, Client],
    get_started: Union[Unset, None, bool] = UNSET,

) -> Optional[GetCompletedJobResultMaybeResponse200]:
    """ get completed job result if job is completed

    Args:
        workspace (str):
        id (str):
        get_started (Union[Unset, None, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetCompletedJobResultMaybeResponse200
     """


    return sync_detailed(
        workspace=workspace,
id=id,
client=client,
get_started=get_started,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    id: str,
    *,
    client: Union[AuthenticatedClient, Client],
    get_started: Union[Unset, None, bool] = UNSET,

) -> Response[GetCompletedJobResultMaybeResponse200]:
    """ get completed job result if job is completed

    Args:
        workspace (str):
        id (str):
        get_started (Union[Unset, None, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetCompletedJobResultMaybeResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
id=id,
get_started=get_started,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    workspace: str,
    id: str,
    *,
    client: Union[AuthenticatedClient, Client],
    get_started: Union[Unset, None, bool] = UNSET,

) -> Optional[GetCompletedJobResultMaybeResponse200]:
    """ get completed job result if job is completed

    Args:
        workspace (str):
        id (str):
        get_started (Union[Unset, None, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetCompletedJobResultMaybeResponse200
     """


    return (await asyncio_detailed(
        workspace=workspace,
id=id,
client=client,
get_started=get_started,

    )).parsed
