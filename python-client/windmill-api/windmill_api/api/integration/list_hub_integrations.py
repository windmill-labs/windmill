from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import Union
from ...models.list_hub_integrations_response_200_item import ListHubIntegrationsResponse200Item
from typing import cast
from typing import cast, List
from typing import Dict
from typing import Optional
from ...types import UNSET, Unset



def _get_kwargs(
    *,
    kind: Union[Unset, None, str] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["kind"] = kind



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    

    

    return {
        "method": "get",
        "url": "/integrations/hub/list",
        "params": params,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[List['ListHubIntegrationsResponse200Item']]:
    if response.status_code == HTTPStatus.OK:
        response_200 = []
        _response_200 = response.json()
        for response_200_item_data in (_response_200):
            response_200_item = ListHubIntegrationsResponse200Item.from_dict(response_200_item_data)



            response_200.append(response_200_item)

        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[List['ListHubIntegrationsResponse200Item']]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: Union[AuthenticatedClient, Client],
    kind: Union[Unset, None, str] = UNSET,

) -> Response[List['ListHubIntegrationsResponse200Item']]:
    """ list hub integrations

    Args:
        kind (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[List['ListHubIntegrationsResponse200Item']]
     """


    kwargs = _get_kwargs(
        kind=kind,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    *,
    client: Union[AuthenticatedClient, Client],
    kind: Union[Unset, None, str] = UNSET,

) -> Optional[List['ListHubIntegrationsResponse200Item']]:
    """ list hub integrations

    Args:
        kind (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        List['ListHubIntegrationsResponse200Item']
     """


    return sync_detailed(
        client=client,
kind=kind,

    ).parsed

async def asyncio_detailed(
    *,
    client: Union[AuthenticatedClient, Client],
    kind: Union[Unset, None, str] = UNSET,

) -> Response[List['ListHubIntegrationsResponse200Item']]:
    """ list hub integrations

    Args:
        kind (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[List['ListHubIntegrationsResponse200Item']]
     """


    kwargs = _get_kwargs(
        kind=kind,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    *,
    client: Union[AuthenticatedClient, Client],
    kind: Union[Unset, None, str] = UNSET,

) -> Optional[List['ListHubIntegrationsResponse200Item']]:
    """ list hub integrations

    Args:
        kind (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        List['ListHubIntegrationsResponse200Item']
     """


    return (await asyncio_detailed(
        client=client,
kind=kind,

    )).parsed
