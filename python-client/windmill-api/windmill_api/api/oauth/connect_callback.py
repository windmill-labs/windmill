from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import cast
from ...models.connect_callback_response_200 import ConnectCallbackResponse200
from typing import Dict
from ...models.connect_callback_json_body import ConnectCallbackJsonBody



def _get_kwargs(
    client_name: str,
    *,
    json_body: ConnectCallbackJsonBody,

) -> Dict[str, Any]:
    

    cookies = {}


    

    json_json_body = json_body.to_dict()



    

    return {
        "method": "post",
        "url": "/oauth/connect_callback/{client_name}".format(client_name=client_name,),
        "json": json_json_body,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[ConnectCallbackResponse200]:
    if response.status_code == HTTPStatus.OK:
        response_200 = ConnectCallbackResponse200.from_dict(response.json())



        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[ConnectCallbackResponse200]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    client_name: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: ConnectCallbackJsonBody,

) -> Response[ConnectCallbackResponse200]:
    """ connect callback

    Args:
        client_name (str):
        json_body (ConnectCallbackJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ConnectCallbackResponse200]
     """


    kwargs = _get_kwargs(
        client_name=client_name,
json_body=json_body,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    client_name: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: ConnectCallbackJsonBody,

) -> Optional[ConnectCallbackResponse200]:
    """ connect callback

    Args:
        client_name (str):
        json_body (ConnectCallbackJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ConnectCallbackResponse200
     """


    return sync_detailed(
        client_name=client_name,
client=client,
json_body=json_body,

    ).parsed

async def asyncio_detailed(
    client_name: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: ConnectCallbackJsonBody,

) -> Response[ConnectCallbackResponse200]:
    """ connect callback

    Args:
        client_name (str):
        json_body (ConnectCallbackJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ConnectCallbackResponse200]
     """


    kwargs = _get_kwargs(
        client_name=client_name,
json_body=json_body,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    client_name: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: ConnectCallbackJsonBody,

) -> Optional[ConnectCallbackResponse200]:
    """ connect callback

    Args:
        client_name (str):
        json_body (ConnectCallbackJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ConnectCallbackResponse200
     """


    return (await asyncio_detailed(
        client_name=client_name,
client=client,
json_body=json_body,

    )).parsed
