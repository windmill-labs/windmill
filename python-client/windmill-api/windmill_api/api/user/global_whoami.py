from typing import Any, Dict, Optional

import httpx

from ...client import Client
from ...models.global_user_info import GlobalUserInfo
from ...types import Response


def _get_kwargs(
    *,
    client: Client,
) -> Dict[str, Any]:
    url = "{}/users/whoami".format(client.base_url)

    headers: Dict[str, str] = client.get_headers()
    cookies: Dict[str, Any] = client.get_cookies()

    return {
        "method": "get",
        "url": url,
        "headers": headers,
        "cookies": cookies,
        "timeout": client.get_timeout(),
    }


def _parse_response(*, response: httpx.Response) -> Optional[GlobalUserInfo]:
    if response.status_code == 200:
        response_200 = GlobalUserInfo.from_dict(response.json())

        return response_200
    return None


def _build_response(*, response: httpx.Response) -> Response[GlobalUserInfo]:
    return Response(
        status_code=response.status_code,
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(response=response),
    )


def sync_detailed(
    *,
    client: Client,
) -> Response[GlobalUserInfo]:
    """get current global whoami (if logged in)

    Returns:
        Response[GlobalUserInfo]
    """

    kwargs = _get_kwargs(
        client=client,
    )

    response = httpx.request(
        verify=client.verify_ssl,
        **kwargs,
    )

    return _build_response(response=response)


def sync(
    *,
    client: Client,
) -> Optional[GlobalUserInfo]:
    """get current global whoami (if logged in)

    Returns:
        Response[GlobalUserInfo]
    """

    return sync_detailed(
        client=client,
    ).parsed


async def asyncio_detailed(
    *,
    client: Client,
) -> Response[GlobalUserInfo]:
    """get current global whoami (if logged in)

    Returns:
        Response[GlobalUserInfo]
    """

    kwargs = _get_kwargs(
        client=client,
    )

    async with httpx.AsyncClient(verify=client.verify_ssl) as _client:
        response = await _client.request(**kwargs)

    return _build_response(response=response)


async def asyncio(
    *,
    client: Client,
) -> Optional[GlobalUserInfo]:
    """get current global whoami (if logged in)

    Returns:
        Response[GlobalUserInfo]
    """

    return (
        await asyncio_detailed(
            client=client,
        )
    ).parsed
