from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import cast
from typing import Dict
from ...models.delete_script_by_hash_response_200 import DeleteScriptByHashResponse200



def _get_kwargs(
    workspace: str,
    hash_: str,

) -> Dict[str, Any]:
    

    cookies = {}


    

    

    

    return {
        "method": "post",
        "url": "/w/{workspace}/scripts/delete/h/{hash}".format(workspace=workspace,hash=hash_,),
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[DeleteScriptByHashResponse200]:
    if response.status_code == HTTPStatus.OK:
        response_200 = DeleteScriptByHashResponse200.from_dict(response.json())



        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[DeleteScriptByHashResponse200]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    workspace: str,
    hash_: str,
    *,
    client: Union[AuthenticatedClient, Client],

) -> Response[DeleteScriptByHashResponse200]:
    """ delete script by hash (erase content but keep hash, require admin)

    Args:
        workspace (str):
        hash_ (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[DeleteScriptByHashResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
hash_=hash_,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    workspace: str,
    hash_: str,
    *,
    client: Union[AuthenticatedClient, Client],

) -> Optional[DeleteScriptByHashResponse200]:
    """ delete script by hash (erase content but keep hash, require admin)

    Args:
        workspace (str):
        hash_ (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        DeleteScriptByHashResponse200
     """


    return sync_detailed(
        workspace=workspace,
hash_=hash_,
client=client,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    hash_: str,
    *,
    client: Union[AuthenticatedClient, Client],

) -> Response[DeleteScriptByHashResponse200]:
    """ delete script by hash (erase content but keep hash, require admin)

    Args:
        workspace (str):
        hash_ (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[DeleteScriptByHashResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
hash_=hash_,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    workspace: str,
    hash_: str,
    *,
    client: Union[AuthenticatedClient, Client],

) -> Optional[DeleteScriptByHashResponse200]:
    """ delete script by hash (erase content but keep hash, require admin)

    Args:
        workspace (str):
        hash_ (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        DeleteScriptByHashResponse200
     """


    return (await asyncio_detailed(
        workspace=workspace,
hash_=hash_,
client=client,

    )).parsed
