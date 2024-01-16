from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from ...models.get_granular_acls_kind import GetGranularAclsKind
from typing import cast
from ...models.get_granular_acls_response_200 import GetGranularAclsResponse200
from typing import Dict



def _get_kwargs(
    workspace: str,
    kind: GetGranularAclsKind,
    path: str,

) -> Dict[str, Any]:
    

    cookies = {}


    

    

    

    return {
        "method": "get",
        "url": "/w/{workspace}/acls/get/{kind}/{path}".format(workspace=workspace,kind=kind,path=path,),
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[GetGranularAclsResponse200]:
    if response.status_code == HTTPStatus.OK:
        response_200 = GetGranularAclsResponse200.from_dict(response.json())



        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[GetGranularAclsResponse200]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    workspace: str,
    kind: GetGranularAclsKind,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],

) -> Response[GetGranularAclsResponse200]:
    """ get granular acls

    Args:
        workspace (str):
        kind (GetGranularAclsKind):
        path (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetGranularAclsResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
kind=kind,
path=path,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    workspace: str,
    kind: GetGranularAclsKind,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],

) -> Optional[GetGranularAclsResponse200]:
    """ get granular acls

    Args:
        workspace (str):
        kind (GetGranularAclsKind):
        path (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetGranularAclsResponse200
     """


    return sync_detailed(
        workspace=workspace,
kind=kind,
path=path,
client=client,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    kind: GetGranularAclsKind,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],

) -> Response[GetGranularAclsResponse200]:
    """ get granular acls

    Args:
        workspace (str):
        kind (GetGranularAclsKind):
        path (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetGranularAclsResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
kind=kind,
path=path,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    workspace: str,
    kind: GetGranularAclsKind,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],

) -> Optional[GetGranularAclsResponse200]:
    """ get granular acls

    Args:
        workspace (str):
        kind (GetGranularAclsKind):
        path (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetGranularAclsResponse200
     """


    return (await asyncio_detailed(
        workspace=workspace,
kind=kind,
path=path,
client=client,

    )).parsed
