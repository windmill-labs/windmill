from typing import Any, Dict, List, Optional, Union

import httpx

from ...client import Client
from ...models.list_flows_response_200_item import ListFlowsResponse200Item
from ...types import UNSET, Response, Unset


def _get_kwargs(
    workspace: str,
    *,
    client: Client,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
    order_desc: Union[Unset, None, bool] = UNSET,
    created_by: Union[Unset, None, str] = UNSET,
    path_start: Union[Unset, None, str] = UNSET,
    path_exact: Union[Unset, None, str] = UNSET,
    show_archived: Union[Unset, None, bool] = UNSET,
) -> Dict[str, Any]:
    url = "{}/w/{workspace}/flows/list".format(client.base_url, workspace=workspace)

    headers: Dict[str, str] = client.get_headers()
    cookies: Dict[str, Any] = client.get_cookies()

    params: Dict[str, Any] = {}
    params["page"] = page

    params["per_page"] = per_page

    params["order_desc"] = order_desc

    params["created_by"] = created_by

    params["path_start"] = path_start

    params["path_exact"] = path_exact

    params["show_archived"] = show_archived

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    return {
        "method": "get",
        "url": url,
        "headers": headers,
        "cookies": cookies,
        "timeout": client.get_timeout(),
        "params": params,
    }


def _parse_response(*, response: httpx.Response) -> Optional[List[ListFlowsResponse200Item]]:
    if response.status_code == 200:
        response_200 = []
        _response_200 = response.json()
        for response_200_item_data in _response_200:
            response_200_item = ListFlowsResponse200Item.from_dict(response_200_item_data)

            response_200.append(response_200_item)

        return response_200
    return None


def _build_response(*, response: httpx.Response) -> Response[List[ListFlowsResponse200Item]]:
    return Response(
        status_code=response.status_code,
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(response=response),
    )


def sync_detailed(
    workspace: str,
    *,
    client: Client,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
    order_desc: Union[Unset, None, bool] = UNSET,
    created_by: Union[Unset, None, str] = UNSET,
    path_start: Union[Unset, None, str] = UNSET,
    path_exact: Union[Unset, None, str] = UNSET,
    show_archived: Union[Unset, None, bool] = UNSET,
) -> Response[List[ListFlowsResponse200Item]]:
    """list all available flows

    Args:
        workspace (str):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):
        order_desc (Union[Unset, None, bool]):
        created_by (Union[Unset, None, str]):
        path_start (Union[Unset, None, str]):
        path_exact (Union[Unset, None, str]):
        show_archived (Union[Unset, None, bool]):

    Returns:
        Response[List[ListFlowsResponse200Item]]
    """

    kwargs = _get_kwargs(
        workspace=workspace,
        client=client,
        page=page,
        per_page=per_page,
        order_desc=order_desc,
        created_by=created_by,
        path_start=path_start,
        path_exact=path_exact,
        show_archived=show_archived,
    )

    response = httpx.request(
        verify=client.verify_ssl,
        **kwargs,
    )

    return _build_response(response=response)


def sync(
    workspace: str,
    *,
    client: Client,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
    order_desc: Union[Unset, None, bool] = UNSET,
    created_by: Union[Unset, None, str] = UNSET,
    path_start: Union[Unset, None, str] = UNSET,
    path_exact: Union[Unset, None, str] = UNSET,
    show_archived: Union[Unset, None, bool] = UNSET,
) -> Optional[List[ListFlowsResponse200Item]]:
    """list all available flows

    Args:
        workspace (str):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):
        order_desc (Union[Unset, None, bool]):
        created_by (Union[Unset, None, str]):
        path_start (Union[Unset, None, str]):
        path_exact (Union[Unset, None, str]):
        show_archived (Union[Unset, None, bool]):

    Returns:
        Response[List[ListFlowsResponse200Item]]
    """

    return sync_detailed(
        workspace=workspace,
        client=client,
        page=page,
        per_page=per_page,
        order_desc=order_desc,
        created_by=created_by,
        path_start=path_start,
        path_exact=path_exact,
        show_archived=show_archived,
    ).parsed


async def asyncio_detailed(
    workspace: str,
    *,
    client: Client,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
    order_desc: Union[Unset, None, bool] = UNSET,
    created_by: Union[Unset, None, str] = UNSET,
    path_start: Union[Unset, None, str] = UNSET,
    path_exact: Union[Unset, None, str] = UNSET,
    show_archived: Union[Unset, None, bool] = UNSET,
) -> Response[List[ListFlowsResponse200Item]]:
    """list all available flows

    Args:
        workspace (str):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):
        order_desc (Union[Unset, None, bool]):
        created_by (Union[Unset, None, str]):
        path_start (Union[Unset, None, str]):
        path_exact (Union[Unset, None, str]):
        show_archived (Union[Unset, None, bool]):

    Returns:
        Response[List[ListFlowsResponse200Item]]
    """

    kwargs = _get_kwargs(
        workspace=workspace,
        client=client,
        page=page,
        per_page=per_page,
        order_desc=order_desc,
        created_by=created_by,
        path_start=path_start,
        path_exact=path_exact,
        show_archived=show_archived,
    )

    async with httpx.AsyncClient(verify=client.verify_ssl) as _client:
        response = await _client.request(**kwargs)

    return _build_response(response=response)


async def asyncio(
    workspace: str,
    *,
    client: Client,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
    order_desc: Union[Unset, None, bool] = UNSET,
    created_by: Union[Unset, None, str] = UNSET,
    path_start: Union[Unset, None, str] = UNSET,
    path_exact: Union[Unset, None, str] = UNSET,
    show_archived: Union[Unset, None, bool] = UNSET,
) -> Optional[List[ListFlowsResponse200Item]]:
    """list all available flows

    Args:
        workspace (str):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):
        order_desc (Union[Unset, None, bool]):
        created_by (Union[Unset, None, str]):
        path_start (Union[Unset, None, str]):
        path_exact (Union[Unset, None, str]):
        show_archived (Union[Unset, None, bool]):

    Returns:
        Response[List[ListFlowsResponse200Item]]
    """

    return (
        await asyncio_detailed(
            workspace=workspace,
            client=client,
            page=page,
            per_page=per_page,
            order_desc=order_desc,
            created_by=created_by,
            path_start=path_start,
            path_exact=path_exact,
            show_archived=show_archived,
        )
    ).parsed
