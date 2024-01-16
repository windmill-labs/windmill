from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import Union
from typing import cast
from typing import Dict
from ...models.list_stored_files_response_200 import ListStoredFilesResponse200
from typing import Optional
from ...types import UNSET, Unset



def _get_kwargs(
    workspace: str,
    *,
    max_keys: int,
    marker: Union[Unset, None, str] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["max_keys"] = max_keys


    params["marker"] = marker



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    

    

    return {
        "method": "get",
        "url": "/w/{workspace}/job_helpers/list_stored_files".format(workspace=workspace,),
        "params": params,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[ListStoredFilesResponse200]:
    if response.status_code == HTTPStatus.OK:
        response_200 = ListStoredFilesResponse200.from_dict(response.json())



        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[ListStoredFilesResponse200]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    max_keys: int,
    marker: Union[Unset, None, str] = UNSET,

) -> Response[ListStoredFilesResponse200]:
    """ List the file keys available in the workspace files storage (S3)

    Args:
        workspace (str):
        max_keys (int):
        marker (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ListStoredFilesResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
max_keys=max_keys,
marker=marker,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    max_keys: int,
    marker: Union[Unset, None, str] = UNSET,

) -> Optional[ListStoredFilesResponse200]:
    """ List the file keys available in the workspace files storage (S3)

    Args:
        workspace (str):
        max_keys (int):
        marker (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ListStoredFilesResponse200
     """


    return sync_detailed(
        workspace=workspace,
client=client,
max_keys=max_keys,
marker=marker,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    max_keys: int,
    marker: Union[Unset, None, str] = UNSET,

) -> Response[ListStoredFilesResponse200]:
    """ List the file keys available in the workspace files storage (S3)

    Args:
        workspace (str):
        max_keys (int):
        marker (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ListStoredFilesResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
max_keys=max_keys,
marker=marker,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    max_keys: int,
    marker: Union[Unset, None, str] = UNSET,

) -> Optional[ListStoredFilesResponse200]:
    """ List the file keys available in the workspace files storage (S3)

    Args:
        workspace (str):
        max_keys (int):
        marker (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ListStoredFilesResponse200
     """


    return (await asyncio_detailed(
        workspace=workspace,
client=client,
max_keys=max_keys,
marker=marker,

    )).parsed
