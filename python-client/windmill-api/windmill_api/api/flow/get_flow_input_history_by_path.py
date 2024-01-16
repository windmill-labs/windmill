from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import Union
from typing import cast
from typing import cast, List
from typing import Dict
from ...models.get_flow_input_history_by_path_response_200_item import GetFlowInputHistoryByPathResponse200Item
from typing import Optional
from ...types import UNSET, Unset



def _get_kwargs(
    workspace: str,
    path: str,
    *,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["page"] = page


    params["per_page"] = per_page



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    

    

    return {
        "method": "get",
        "url": "/w/{workspace}/flows/input_history/p/{path}".format(workspace=workspace,path=path,),
        "params": params,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[List['GetFlowInputHistoryByPathResponse200Item']]:
    if response.status_code == HTTPStatus.OK:
        response_200 = []
        _response_200 = response.json()
        for response_200_item_data in (_response_200):
            response_200_item = GetFlowInputHistoryByPathResponse200Item.from_dict(response_200_item_data)



            response_200.append(response_200_item)

        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[List['GetFlowInputHistoryByPathResponse200Item']]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    workspace: str,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,

) -> Response[List['GetFlowInputHistoryByPathResponse200Item']]:
    """ list inputs for previous completed flow jobs

    Args:
        workspace (str):
        path (str):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[List['GetFlowInputHistoryByPathResponse200Item']]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
path=path,
page=page,
per_page=per_page,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    workspace: str,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,

) -> Optional[List['GetFlowInputHistoryByPathResponse200Item']]:
    """ list inputs for previous completed flow jobs

    Args:
        workspace (str):
        path (str):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        List['GetFlowInputHistoryByPathResponse200Item']
     """


    return sync_detailed(
        workspace=workspace,
path=path,
client=client,
page=page,
per_page=per_page,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,

) -> Response[List['GetFlowInputHistoryByPathResponse200Item']]:
    """ list inputs for previous completed flow jobs

    Args:
        workspace (str):
        path (str):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[List['GetFlowInputHistoryByPathResponse200Item']]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
path=path,
page=page,
per_page=per_page,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    workspace: str,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,

) -> Optional[List['GetFlowInputHistoryByPathResponse200Item']]:
    """ list inputs for previous completed flow jobs

    Args:
        workspace (str):
        path (str):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        List['GetFlowInputHistoryByPathResponse200Item']
     """


    return (await asyncio_detailed(
        workspace=workspace,
path=path,
client=client,
page=page,
per_page=per_page,

    )).parsed
