from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import Union
from typing import cast
from typing import Dict
from ...models.get_variable_response_200 import GetVariableResponse200
from typing import Optional
from ...types import UNSET, Unset



def _get_kwargs(
    workspace: str,
    path: str,
    *,
    decrypt_secret: Union[Unset, None, bool] = UNSET,
    include_encrypted: Union[Unset, None, bool] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["decrypt_secret"] = decrypt_secret


    params["include_encrypted"] = include_encrypted



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    

    

    return {
        "method": "get",
        "url": "/w/{workspace}/variables/get/{path}".format(workspace=workspace,path=path,),
        "params": params,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[GetVariableResponse200]:
    if response.status_code == HTTPStatus.OK:
        response_200 = GetVariableResponse200.from_dict(response.json())



        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[GetVariableResponse200]:
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
    decrypt_secret: Union[Unset, None, bool] = UNSET,
    include_encrypted: Union[Unset, None, bool] = UNSET,

) -> Response[GetVariableResponse200]:
    """ get variable

    Args:
        workspace (str):
        path (str):
        decrypt_secret (Union[Unset, None, bool]):
        include_encrypted (Union[Unset, None, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetVariableResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
path=path,
decrypt_secret=decrypt_secret,
include_encrypted=include_encrypted,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    workspace: str,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],
    decrypt_secret: Union[Unset, None, bool] = UNSET,
    include_encrypted: Union[Unset, None, bool] = UNSET,

) -> Optional[GetVariableResponse200]:
    """ get variable

    Args:
        workspace (str):
        path (str):
        decrypt_secret (Union[Unset, None, bool]):
        include_encrypted (Union[Unset, None, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetVariableResponse200
     """


    return sync_detailed(
        workspace=workspace,
path=path,
client=client,
decrypt_secret=decrypt_secret,
include_encrypted=include_encrypted,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],
    decrypt_secret: Union[Unset, None, bool] = UNSET,
    include_encrypted: Union[Unset, None, bool] = UNSET,

) -> Response[GetVariableResponse200]:
    """ get variable

    Args:
        workspace (str):
        path (str):
        decrypt_secret (Union[Unset, None, bool]):
        include_encrypted (Union[Unset, None, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetVariableResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
path=path,
decrypt_secret=decrypt_secret,
include_encrypted=include_encrypted,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    workspace: str,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],
    decrypt_secret: Union[Unset, None, bool] = UNSET,
    include_encrypted: Union[Unset, None, bool] = UNSET,

) -> Optional[GetVariableResponse200]:
    """ get variable

    Args:
        workspace (str):
        path (str):
        decrypt_secret (Union[Unset, None, bool]):
        include_encrypted (Union[Unset, None, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetVariableResponse200
     """


    return (await asyncio_detailed(
        workspace=workspace,
path=path,
client=client,
decrypt_secret=decrypt_secret,
include_encrypted=include_encrypted,

    )).parsed
