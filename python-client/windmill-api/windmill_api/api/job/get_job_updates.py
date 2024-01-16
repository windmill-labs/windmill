from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import Union
from typing import cast
from typing import Dict
from ...models.get_job_updates_response_200 import GetJobUpdatesResponse200
from typing import Optional
from ...types import UNSET, Unset



def _get_kwargs(
    workspace: str,
    id: str,
    *,
    running: Union[Unset, None, bool] = UNSET,
    log_offset: Union[Unset, None, int] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["running"] = running


    params["log_offset"] = log_offset



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    

    

    return {
        "method": "get",
        "url": "/w/{workspace}/jobs_u/getupdate/{id}".format(workspace=workspace,id=id,),
        "params": params,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[GetJobUpdatesResponse200]:
    if response.status_code == HTTPStatus.OK:
        response_200 = GetJobUpdatesResponse200.from_dict(response.json())



        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[GetJobUpdatesResponse200]:
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
    running: Union[Unset, None, bool] = UNSET,
    log_offset: Union[Unset, None, int] = UNSET,

) -> Response[GetJobUpdatesResponse200]:
    """ get job updates

    Args:
        workspace (str):
        id (str):
        running (Union[Unset, None, bool]):
        log_offset (Union[Unset, None, int]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetJobUpdatesResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
id=id,
running=running,
log_offset=log_offset,

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
    running: Union[Unset, None, bool] = UNSET,
    log_offset: Union[Unset, None, int] = UNSET,

) -> Optional[GetJobUpdatesResponse200]:
    """ get job updates

    Args:
        workspace (str):
        id (str):
        running (Union[Unset, None, bool]):
        log_offset (Union[Unset, None, int]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetJobUpdatesResponse200
     """


    return sync_detailed(
        workspace=workspace,
id=id,
client=client,
running=running,
log_offset=log_offset,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    id: str,
    *,
    client: Union[AuthenticatedClient, Client],
    running: Union[Unset, None, bool] = UNSET,
    log_offset: Union[Unset, None, int] = UNSET,

) -> Response[GetJobUpdatesResponse200]:
    """ get job updates

    Args:
        workspace (str):
        id (str):
        running (Union[Unset, None, bool]):
        log_offset (Union[Unset, None, int]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetJobUpdatesResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
id=id,
running=running,
log_offset=log_offset,

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
    running: Union[Unset, None, bool] = UNSET,
    log_offset: Union[Unset, None, int] = UNSET,

) -> Optional[GetJobUpdatesResponse200]:
    """ get job updates

    Args:
        workspace (str):
        id (str):
        running (Union[Unset, None, bool]):
        log_offset (Union[Unset, None, int]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetJobUpdatesResponse200
     """


    return (await asyncio_detailed(
        workspace=workspace,
id=id,
client=client,
running=running,
log_offset=log_offset,

    )).parsed
