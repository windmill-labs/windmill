from typing import Any, Dict, Optional

import httpx

from ...client import Client
from ...models.get_hub_flow_by_id_response_200 import GetHubFlowByIdResponse200
from ...types import Response


def _get_kwargs(
    id: int,
    *,
    client: Client,
) -> Dict[str, Any]:
    url = "{}/flows/hub/get/{id}".format(client.base_url, id=id)

    headers: Dict[str, str] = client.get_headers()
    cookies: Dict[str, Any] = client.get_cookies()

    return {
        "method": "get",
        "url": url,
        "headers": headers,
        "cookies": cookies,
        "timeout": client.get_timeout(),
    }


def _parse_response(*, response: httpx.Response) -> Optional[GetHubFlowByIdResponse200]:
    if response.status_code == 200:
        response_200 = GetHubFlowByIdResponse200.from_dict(response.json())

        return response_200
    return None


def _build_response(*, response: httpx.Response) -> Response[GetHubFlowByIdResponse200]:
    return Response(
        status_code=response.status_code,
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(response=response),
    )


def sync_detailed(
    id: int,
    *,
    client: Client,
) -> Response[GetHubFlowByIdResponse200]:
    """get hub flow by id

    Args:
        id (int):

    Returns:
        Response[GetHubFlowByIdResponse200]
    """

    kwargs = _get_kwargs(
        id=id,
        client=client,
    )

    response = httpx.request(
        verify=client.verify_ssl,
        **kwargs,
    )

    return _build_response(response=response)


def sync(
    id: int,
    *,
    client: Client,
) -> Optional[GetHubFlowByIdResponse200]:
    """get hub flow by id

    Args:
        id (int):

    Returns:
        Response[GetHubFlowByIdResponse200]
    """

    return sync_detailed(
        id=id,
        client=client,
    ).parsed


async def asyncio_detailed(
    id: int,
    *,
    client: Client,
) -> Response[GetHubFlowByIdResponse200]:
    """get hub flow by id

    Args:
        id (int):

    Returns:
        Response[GetHubFlowByIdResponse200]
    """

    kwargs = _get_kwargs(
        id=id,
        client=client,
    )

    async with httpx.AsyncClient(verify=client.verify_ssl) as _client:
        response = await _client.request(**kwargs)

    return _build_response(response=response)


async def asyncio(
    id: int,
    *,
    client: Client,
) -> Optional[GetHubFlowByIdResponse200]:
    """get hub flow by id

    Args:
        id (int):

    Returns:
        Response[GetHubFlowByIdResponse200]
    """

    return (
        await asyncio_detailed(
            id=id,
            client=client,
        )
    ).parsed
