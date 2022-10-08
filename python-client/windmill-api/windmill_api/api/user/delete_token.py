from typing import Any, Dict

import httpx

from ...client import Client
from ...types import Response


def _get_kwargs(
    token_prefix: str,
    *,
    client: Client,
) -> Dict[str, Any]:
    url = "{}/users/tokens/delete/{token_prefix}".format(client.base_url, token_prefix=token_prefix)

    headers: Dict[str, str] = client.get_headers()
    cookies: Dict[str, Any] = client.get_cookies()

    return {
        "method": "delete",
        "url": url,
        "headers": headers,
        "cookies": cookies,
        "timeout": client.get_timeout(),
    }


def _build_response(*, response: httpx.Response) -> Response[Any]:
    return Response(
        status_code=response.status_code,
        content=response.content,
        headers=response.headers,
        parsed=None,
    )


def sync_detailed(
    token_prefix: str,
    *,
    client: Client,
) -> Response[Any]:
    """delete token

    Args:
        token_prefix (str):

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs(
        token_prefix=token_prefix,
        client=client,
    )

    response = httpx.request(
        verify=client.verify_ssl,
        **kwargs,
    )

    return _build_response(response=response)


async def asyncio_detailed(
    token_prefix: str,
    *,
    client: Client,
) -> Response[Any]:
    """delete token

    Args:
        token_prefix (str):

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs(
        token_prefix=token_prefix,
        client=client,
    )

    async with httpx.AsyncClient(verify=client.verify_ssl) as _client:
        response = await _client.request(**kwargs)

    return _build_response(response=response)
