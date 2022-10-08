from typing import Any, Dict, Optional

import httpx

from ...client import Client
from ...models.connect_slack_callback_json_body import ConnectSlackCallbackJsonBody
from ...models.connect_slack_callback_response_200 import ConnectSlackCallbackResponse200
from ...types import Response


def _get_kwargs(
    *,
    client: Client,
    json_body: ConnectSlackCallbackJsonBody,
) -> Dict[str, Any]:
    url = "{}/oauth/connect_slack_callback".format(client.base_url)

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


def _parse_response(*, response: httpx.Response) -> Optional[ConnectSlackCallbackResponse200]:
    if response.status_code == 200:
        response_200 = ConnectSlackCallbackResponse200.from_dict(response.json())

        return response_200
    return None


def _build_response(*, response: httpx.Response) -> Response[ConnectSlackCallbackResponse200]:
    return Response(
        status_code=response.status_code,
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(response=response),
    )


def sync_detailed(
    *,
    client: Client,
    json_body: ConnectSlackCallbackJsonBody,
) -> Response[ConnectSlackCallbackResponse200]:
    """connect slack callback

    Args:
        json_body (ConnectSlackCallbackJsonBody):

    Returns:
        Response[ConnectSlackCallbackResponse200]
    """

    kwargs = _get_kwargs(
        client=client,
        json_body=json_body,
    )

    response = httpx.request(
        verify=client.verify_ssl,
        **kwargs,
    )

    return _build_response(response=response)


def sync(
    *,
    client: Client,
    json_body: ConnectSlackCallbackJsonBody,
) -> Optional[ConnectSlackCallbackResponse200]:
    """connect slack callback

    Args:
        json_body (ConnectSlackCallbackJsonBody):

    Returns:
        Response[ConnectSlackCallbackResponse200]
    """

    return sync_detailed(
        client=client,
        json_body=json_body,
    ).parsed


async def asyncio_detailed(
    *,
    client: Client,
    json_body: ConnectSlackCallbackJsonBody,
) -> Response[ConnectSlackCallbackResponse200]:
    """connect slack callback

    Args:
        json_body (ConnectSlackCallbackJsonBody):

    Returns:
        Response[ConnectSlackCallbackResponse200]
    """

    kwargs = _get_kwargs(
        client=client,
        json_body=json_body,
    )

    async with httpx.AsyncClient(verify=client.verify_ssl) as _client:
        response = await _client.request(**kwargs)

    return _build_response(response=response)


async def asyncio(
    *,
    client: Client,
    json_body: ConnectSlackCallbackJsonBody,
) -> Optional[ConnectSlackCallbackResponse200]:
    """connect slack callback

    Args:
        json_body (ConnectSlackCallbackJsonBody):

    Returns:
        Response[ConnectSlackCallbackResponse200]
    """

    return (
        await asyncio_detailed(
            client=client,
            json_body=json_body,
        )
    ).parsed
