from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import cast
from ...models.get_group_response_200 import GetGroupResponse200
from typing import Dict



def _get_kwargs(
    workspace: str,
    name: str,

) -> Dict[str, Any]:
    

    cookies = {}


    

    

    

    return {
        "method": "get",
        "url": "/w/{workspace}/groups/get/{name}".format(workspace=workspace,name=name,),
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[GetGroupResponse200]:
    if response.status_code == HTTPStatus.OK:
        response_200 = GetGroupResponse200.from_dict(response.json())



        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[GetGroupResponse200]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    workspace: str,
    name: str,
    *,
    client: Union[AuthenticatedClient, Client],

) -> Response[GetGroupResponse200]:
    """ get group

    Args:
        workspace (str):
        name (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetGroupResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
name=name,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    workspace: str,
    name: str,
    *,
    client: Union[AuthenticatedClient, Client],

) -> Optional[GetGroupResponse200]:
    """ get group

    Args:
        workspace (str):
        name (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetGroupResponse200
     """


    return sync_detailed(
        workspace=workspace,
name=name,
client=client,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    name: str,
    *,
    client: Union[AuthenticatedClient, Client],

) -> Response[GetGroupResponse200]:
    """ get group

    Args:
        workspace (str):
        name (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetGroupResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
name=name,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    workspace: str,
    name: str,
    *,
    client: Union[AuthenticatedClient, Client],

) -> Optional[GetGroupResponse200]:
    """ get group

    Args:
        workspace (str):
        name (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetGroupResponse200
     """


    return (await asyncio_detailed(
        workspace=workspace,
name=name,
client=client,

    )).parsed
