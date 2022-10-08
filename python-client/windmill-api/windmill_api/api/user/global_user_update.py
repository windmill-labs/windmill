from typing import Any, Dict

import httpx

from ...client import Client
from ...models.global_user_update_json_body import GlobalUserUpdateJsonBody
from ...types import Response


def _get_kwargs(
    email: str,
    *,
    client: Client,
    json_body: GlobalUserUpdateJsonBody,
) -> Dict[str, Any]:
    url = "{}/users/update/{email}".format(client.base_url, email=email)

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


def _build_response(*, response: httpx.Response) -> Response[Any]:
    return Response(
        status_code=response.status_code,
        content=response.content,
        headers=response.headers,
        parsed=None,
    )


def sync_detailed(
    email: str,
    *,
    client: Client,
    json_body: GlobalUserUpdateJsonBody,
) -> Response[Any]:
    """global update user (require super admin)

    Args:
        email (str):
        json_body (GlobalUserUpdateJsonBody):

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs(
        email=email,
        client=client,
        json_body=json_body,
    )

    response = httpx.request(
        verify=client.verify_ssl,
        **kwargs,
    )

    return _build_response(response=response)


async def asyncio_detailed(
    email: str,
    *,
    client: Client,
    json_body: GlobalUserUpdateJsonBody,
) -> Response[Any]:
    """global update user (require super admin)

    Args:
        email (str):
        json_body (GlobalUserUpdateJsonBody):

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs(
        email=email,
        client=client,
        json_body=json_body,
    )

    async with httpx.AsyncClient(verify=client.verify_ssl) as _client:
        response = await _client.request(**kwargs)

    return _build_response(response=response)
