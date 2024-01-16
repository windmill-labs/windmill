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
from ...models.get_input_history_response_200_item import GetInputHistoryResponse200Item
from typing import Optional
from ...types import UNSET, Unset
from ...models.get_input_history_runnable_type import GetInputHistoryRunnableType



def _get_kwargs(
    workspace: str,
    *,
    runnable_id: Union[Unset, None, str] = UNSET,
    runnable_type: Union[Unset, None, GetInputHistoryRunnableType] = UNSET,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["runnable_id"] = runnable_id


    json_runnable_type: Union[Unset, None, str] = UNSET
    if not isinstance(runnable_type, Unset):
        json_runnable_type = runnable_type.value if runnable_type else None

    params["runnable_type"] = json_runnable_type


    params["page"] = page


    params["per_page"] = per_page



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    

    

    return {
        "method": "get",
        "url": "/w/{workspace}/inputs/history".format(workspace=workspace,),
        "params": params,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[List['GetInputHistoryResponse200Item']]:
    if response.status_code == HTTPStatus.OK:
        response_200 = []
        _response_200 = response.json()
        for response_200_item_data in (_response_200):
            response_200_item = GetInputHistoryResponse200Item.from_dict(response_200_item_data)



            response_200.append(response_200_item)

        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[List['GetInputHistoryResponse200Item']]:
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
    runnable_id: Union[Unset, None, str] = UNSET,
    runnable_type: Union[Unset, None, GetInputHistoryRunnableType] = UNSET,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,

) -> Response[List['GetInputHistoryResponse200Item']]:
    """ List Inputs used in previously completed jobs

    Args:
        workspace (str):
        runnable_id (Union[Unset, None, str]):
        runnable_type (Union[Unset, None, GetInputHistoryRunnableType]):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[List['GetInputHistoryResponse200Item']]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
runnable_id=runnable_id,
runnable_type=runnable_type,
page=page,
per_page=per_page,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    runnable_id: Union[Unset, None, str] = UNSET,
    runnable_type: Union[Unset, None, GetInputHistoryRunnableType] = UNSET,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,

) -> Optional[List['GetInputHistoryResponse200Item']]:
    """ List Inputs used in previously completed jobs

    Args:
        workspace (str):
        runnable_id (Union[Unset, None, str]):
        runnable_type (Union[Unset, None, GetInputHistoryRunnableType]):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        List['GetInputHistoryResponse200Item']
     """


    return sync_detailed(
        workspace=workspace,
client=client,
runnable_id=runnable_id,
runnable_type=runnable_type,
page=page,
per_page=per_page,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    runnable_id: Union[Unset, None, str] = UNSET,
    runnable_type: Union[Unset, None, GetInputHistoryRunnableType] = UNSET,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,

) -> Response[List['GetInputHistoryResponse200Item']]:
    """ List Inputs used in previously completed jobs

    Args:
        workspace (str):
        runnable_id (Union[Unset, None, str]):
        runnable_type (Union[Unset, None, GetInputHistoryRunnableType]):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[List['GetInputHistoryResponse200Item']]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
runnable_id=runnable_id,
runnable_type=runnable_type,
page=page,
per_page=per_page,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    runnable_id: Union[Unset, None, str] = UNSET,
    runnable_type: Union[Unset, None, GetInputHistoryRunnableType] = UNSET,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,

) -> Optional[List['GetInputHistoryResponse200Item']]:
    """ List Inputs used in previously completed jobs

    Args:
        workspace (str):
        runnable_id (Union[Unset, None, str]):
        runnable_type (Union[Unset, None, GetInputHistoryRunnableType]):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        List['GetInputHistoryResponse200Item']
     """


    return (await asyncio_detailed(
        workspace=workspace,
client=client,
runnable_id=runnable_id,
runnable_type=runnable_type,
page=page,
per_page=per_page,

    )).parsed
