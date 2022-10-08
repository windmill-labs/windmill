from typing import Any, Dict, Optional

import httpx

from ...client import Client
from ...models.connect_callback_json_body import ConnectCallbackJsonBody
from ...models.connect_callback_response_200 import ConnectCallbackResponse200
from ...types import Response


def _get_kwargs(
    client_name: str,
    *,
    client: Client,
    json_body: ConnectCallbackJsonBody,
) -> Dict[str, Any]:
    url = "{}/oauth/connect_callback/{client_name}".format(client.base_url, client_name=client_name)

    headers: Dict[str, str] = client.get_headers()
    cookies: Dict[str, Any] = client.get_cookies()

    json_json_body = json_body.to_dict()

    return {
        "method": "post",
        "url": url,
        "headers": headers,
        "cookies": cookies,
        "timeout": client.get_timeout(),
        "json": json_json_body,
    }


def _parse_response(*, response: httpx.Response) -> Optional[ConnectCallbackResponse200]:
    if response.status_code == 200:
        response_200 = ConnectCallbackResponse200.from_dict(response.json())

        return response_200
    return None


def _build_response(*, response: httpx.Response) -> Response[ConnectCallbackResponse200]:
    return Response(
        status_code=response.status_code,
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(response=response),
    )


def sync_detailed(
    client_name: str,
    *,
    client: Client,
    json_body: ConnectCallbackJsonBody,
) -> Response[ConnectCallbackResponse200]:
    """connect callback

    Args:
        client_name (str):
        json_body (ConnectCallbackJsonBody):

    Returns:
        Response[ConnectCallbackResponse200]
    """

    kwargs = _get_kwargs(
        client_name=client_name,
        client=client,
        json_body=json_body,
    )

    response = httpx.request(
        verify=client.verify_ssl,
        **kwargs,
    )

    return _build_response(response=response)


def sync(
    client_name: str,
    *,
    client: Client,
    json_body: ConnectCallbackJsonBody,
) -> Optional[ConnectCallbackResponse200]:
    """connect callback

    Args:
        client_name (str):
        json_body (ConnectCallbackJsonBody):

    Returns:
        Response[ConnectCallbackResponse200]
    """

    return sync_detailed(
        client_name=client_name,
        client=client,
        json_body=json_body,
    ).parsed


async def asyncio_detailed(
    client_name: str,
    *,
    client: Client,
    json_body: ConnectCallbackJsonBody,
) -> Response[ConnectCallbackResponse200]:
    """connect callback

    Args:
        client_name (str):
        json_body (ConnectCallbackJsonBody):

    Returns:
        Response[ConnectCallbackResponse200]
    """

    kwargs = _get_kwargs(
        client_name=client_name,
        client=client,
        json_body=json_body,
    )

    async with httpx.AsyncClient(verify=client.verify_ssl) as _client:
        response = await _client.request(**kwargs)

    return _build_response(response=response)


async def asyncio(
    client_name: str,
    *,
    client: Client,
    json_body: ConnectCallbackJsonBody,
) -> Optional[ConnectCallbackResponse200]:
    """connect callback

    Args:
        client_name (str):
        json_body (ConnectCallbackJsonBody):

    Returns:
        Response[ConnectCallbackResponse200]
    """

    return (
        await asyncio_detailed(
            client_name=client_name,
            client=client,
            json_body=json_body,
        )
    ).parsed
