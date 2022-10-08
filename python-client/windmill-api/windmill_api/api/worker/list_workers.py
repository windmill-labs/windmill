from typing import Any, Dict, List, Optional, Union

import httpx

from ...client import Client
from ...models.list_workers_response_200_item import ListWorkersResponse200Item
from ...types import UNSET, Response, Unset


def _get_kwargs(
    *,
    client: Client,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
) -> Dict[str, Any]:
    url = "{}/workers/list".format(client.base_url)

    headers: Dict[str, str] = client.get_headers()
    cookies: Dict[str, Any] = client.get_cookies()

    params: Dict[str, Any] = {}
    params["page"] = page

    params["per_page"] = per_page

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    return {
        "method": "get",
        "url": url,
        "headers": headers,
        "cookies": cookies,
        "timeout": client.get_timeout(),
        "params": params,
    }


def _parse_response(*, response: httpx.Response) -> Optional[List[ListWorkersResponse200Item]]:
    if response.status_code == 200:
        response_200 = []
        _response_200 = response.json()
        for response_200_item_data in _response_200:
            response_200_item = ListWorkersResponse200Item.from_dict(response_200_item_data)

            response_200.append(response_200_item)

        return response_200
    return None


def _build_response(*, response: httpx.Response) -> Response[List[ListWorkersResponse200Item]]:
    return Response(
        status_code=response.status_code,
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(response=response),
    )


def sync_detailed(
    *,
    client: Client,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
) -> Response[List[ListWorkersResponse200Item]]:
    """list workers

    Args:
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):

    Returns:
        Response[List[ListWorkersResponse200Item]]
    """

    kwargs = _get_kwargs(
        client=client,
        page=page,
        per_page=per_page,
    )

    response = httpx.request(
        verify=client.verify_ssl,
        **kwargs,
    )

    return _build_response(response=response)


def sync(
    *,
    client: Client,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
) -> Optional[List[ListWorkersResponse200Item]]:
    """list workers

    Args:
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):

    Returns:
        Response[List[ListWorkersResponse200Item]]
    """

    return sync_detailed(
        client=client,
        page=page,
        per_page=per_page,
    ).parsed


async def asyncio_detailed(
    *,
    client: Client,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
) -> Response[List[ListWorkersResponse200Item]]:
    """list workers

    Args:
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):

    Returns:
        Response[List[ListWorkersResponse200Item]]
    """

    kwargs = _get_kwargs(
        client=client,
        page=page,
        per_page=per_page,
    )

    async with httpx.AsyncClient(verify=client.verify_ssl) as _client:
        response = await _client.request(**kwargs)

    return _build_response(response=response)


async def asyncio(
    *,
    client: Client,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
) -> Optional[List[ListWorkersResponse200Item]]:
    """list workers

    Args:
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):

    Returns:
        Response[List[ListWorkersResponse200Item]]
    """

    return (
        await asyncio_detailed(
            client=client,
            page=page,
            per_page=per_page,
        )
    ).parsed
