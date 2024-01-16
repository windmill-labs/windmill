from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import Union
from typing import cast
from typing import cast, List
from ...models.list_apps_response_200_item import ListAppsResponse200Item
from typing import Dict
from typing import Optional
from ...types import UNSET, Unset



def _get_kwargs(
    workspace: str,
    *,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
    order_desc: Union[Unset, None, bool] = UNSET,
    created_by: Union[Unset, None, str] = UNSET,
    path_start: Union[Unset, None, str] = UNSET,
    path_exact: Union[Unset, None, str] = UNSET,
    starred_only: Union[Unset, None, bool] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["page"] = page


    params["per_page"] = per_page


    params["order_desc"] = order_desc


    params["created_by"] = created_by


    params["path_start"] = path_start


    params["path_exact"] = path_exact


    params["starred_only"] = starred_only



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    

    

    return {
        "method": "get",
        "url": "/w/{workspace}/apps/list".format(workspace=workspace,),
        "params": params,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[List['ListAppsResponse200Item']]:
    if response.status_code == HTTPStatus.OK:
        response_200 = []
        _response_200 = response.json()
        for response_200_item_data in (_response_200):
            response_200_item = ListAppsResponse200Item.from_dict(response_200_item_data)



            response_200.append(response_200_item)

        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[List['ListAppsResponse200Item']]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
    order_desc: Union[Unset, None, bool] = UNSET,
    created_by: Union[Unset, None, str] = UNSET,
    path_start: Union[Unset, None, str] = UNSET,
    path_exact: Union[Unset, None, str] = UNSET,
    starred_only: Union[Unset, None, bool] = UNSET,

) -> Response[List['ListAppsResponse200Item']]:
    """ list all apps

    Args:
        workspace (str):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):
        order_desc (Union[Unset, None, bool]):
        created_by (Union[Unset, None, str]):
        path_start (Union[Unset, None, str]):
        path_exact (Union[Unset, None, str]):
        starred_only (Union[Unset, None, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[List['ListAppsResponse200Item']]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
page=page,
per_page=per_page,
order_desc=order_desc,
created_by=created_by,
path_start=path_start,
path_exact=path_exact,
starred_only=starred_only,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
    order_desc: Union[Unset, None, bool] = UNSET,
    created_by: Union[Unset, None, str] = UNSET,
    path_start: Union[Unset, None, str] = UNSET,
    path_exact: Union[Unset, None, str] = UNSET,
    starred_only: Union[Unset, None, bool] = UNSET,

) -> Optional[List['ListAppsResponse200Item']]:
    """ list all apps

    Args:
        workspace (str):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):
        order_desc (Union[Unset, None, bool]):
        created_by (Union[Unset, None, str]):
        path_start (Union[Unset, None, str]):
        path_exact (Union[Unset, None, str]):
        starred_only (Union[Unset, None, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        List['ListAppsResponse200Item']
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
starred_only=starred_only,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
    order_desc: Union[Unset, None, bool] = UNSET,
    created_by: Union[Unset, None, str] = UNSET,
    path_start: Union[Unset, None, str] = UNSET,
    path_exact: Union[Unset, None, str] = UNSET,
    starred_only: Union[Unset, None, bool] = UNSET,

) -> Response[List['ListAppsResponse200Item']]:
    """ list all apps

    Args:
        workspace (str):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):
        order_desc (Union[Unset, None, bool]):
        created_by (Union[Unset, None, str]):
        path_start (Union[Unset, None, str]):
        path_exact (Union[Unset, None, str]):
        starred_only (Union[Unset, None, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[List['ListAppsResponse200Item']]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
page=page,
per_page=per_page,
order_desc=order_desc,
created_by=created_by,
path_start=path_start,
path_exact=path_exact,
starred_only=starred_only,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
    order_desc: Union[Unset, None, bool] = UNSET,
    created_by: Union[Unset, None, str] = UNSET,
    path_start: Union[Unset, None, str] = UNSET,
    path_exact: Union[Unset, None, str] = UNSET,
    starred_only: Union[Unset, None, bool] = UNSET,

) -> Optional[List['ListAppsResponse200Item']]:
    """ list all apps

    Args:
        workspace (str):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):
        order_desc (Union[Unset, None, bool]):
        created_by (Union[Unset, None, str]):
        path_start (Union[Unset, None, str]):
        path_exact (Union[Unset, None, str]):
        starred_only (Union[Unset, None, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        List['ListAppsResponse200Item']
     """


    return (await asyncio_detailed(
        workspace=workspace,
client=client,
page=page,
per_page=per_page,
order_desc=order_desc,
created_by=created_by,
path_start=path_start,
path_exact=path_exact,
starred_only=starred_only,

    )).parsed
