from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import Union
from typing import cast
from typing import cast, List
from typing import Dict
from typing import Optional
from ...types import UNSET, Unset
from ...models.query_resource_types_response_200_item import QueryResourceTypesResponse200Item



def _get_kwargs(
    workspace: str,
    *,
    text: str,
    limit: Union[Unset, None, float] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["text"] = text


    params["limit"] = limit



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    

    

    return {
        "method": "get",
        "url": "/w/{workspace}/embeddings/query_resource_types".format(workspace=workspace,),
        "params": params,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[List['QueryResourceTypesResponse200Item']]:
    if response.status_code == HTTPStatus.OK:
        response_200 = []
        _response_200 = response.json()
        for response_200_item_data in (_response_200):
            response_200_item = QueryResourceTypesResponse200Item.from_dict(response_200_item_data)



            response_200.append(response_200_item)

        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[List['QueryResourceTypesResponse200Item']]:
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
    text: str,
    limit: Union[Unset, None, float] = UNSET,

) -> Response[List['QueryResourceTypesResponse200Item']]:
    """ query resource types by similarity

    Args:
        workspace (str):
        text (str):
        limit (Union[Unset, None, float]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[List['QueryResourceTypesResponse200Item']]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
text=text,
limit=limit,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    text: str,
    limit: Union[Unset, None, float] = UNSET,

) -> Optional[List['QueryResourceTypesResponse200Item']]:
    """ query resource types by similarity

    Args:
        workspace (str):
        text (str):
        limit (Union[Unset, None, float]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        List['QueryResourceTypesResponse200Item']
     """


    return sync_detailed(
        workspace=workspace,
client=client,
text=text,
limit=limit,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    text: str,
    limit: Union[Unset, None, float] = UNSET,

) -> Response[List['QueryResourceTypesResponse200Item']]:
    """ query resource types by similarity

    Args:
        workspace (str):
        text (str):
        limit (Union[Unset, None, float]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[List['QueryResourceTypesResponse200Item']]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
text=text,
limit=limit,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    text: str,
    limit: Union[Unset, None, float] = UNSET,

) -> Optional[List['QueryResourceTypesResponse200Item']]:
    """ query resource types by similarity

    Args:
        workspace (str):
        text (str):
        limit (Union[Unset, None, float]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        List['QueryResourceTypesResponse200Item']
     """


    return (await asyncio_detailed(
        workspace=workspace,
client=client,
text=text,
limit=limit,

    )).parsed
