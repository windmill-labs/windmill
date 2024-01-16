from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import Union
from typing import cast
from typing import Dict
from ...models.create_input_runnable_type import CreateInputRunnableType
from ...models.create_input_json_body import CreateInputJsonBody
from typing import Optional
from ...types import UNSET, Unset



def _get_kwargs(
    workspace: str,
    *,
    json_body: CreateInputJsonBody,
    runnable_id: Union[Unset, None, str] = UNSET,
    runnable_type: Union[Unset, None, CreateInputRunnableType] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["runnable_id"] = runnable_id


    json_runnable_type: Union[Unset, None, str] = UNSET
    if not isinstance(runnable_type, Unset):
        json_runnable_type = runnable_type.value if runnable_type else None

    params["runnable_type"] = json_runnable_type



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    json_json_body = json_body.to_dict()



    

    return {
        "method": "post",
        "url": "/w/{workspace}/inputs/create".format(workspace=workspace,),
        "json": json_json_body,
        "params": params,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[Any]:
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[Any]:
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
    json_body: CreateInputJsonBody,
    runnable_id: Union[Unset, None, str] = UNSET,
    runnable_type: Union[Unset, None, CreateInputRunnableType] = UNSET,

) -> Response[Any]:
    """ Create an Input for future use in a script or flow

    Args:
        workspace (str):
        runnable_id (Union[Unset, None, str]):
        runnable_type (Union[Unset, None, CreateInputRunnableType]):
        json_body (CreateInputJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
json_body=json_body,
runnable_id=runnable_id,
runnable_type=runnable_type,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


async def asyncio_detailed(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: CreateInputJsonBody,
    runnable_id: Union[Unset, None, str] = UNSET,
    runnable_type: Union[Unset, None, CreateInputRunnableType] = UNSET,

) -> Response[Any]:
    """ Create an Input for future use in a script or flow

    Args:
        workspace (str):
        runnable_id (Union[Unset, None, str]):
        runnable_type (Union[Unset, None, CreateInputRunnableType]):
        json_body (CreateInputJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
json_body=json_body,
runnable_id=runnable_id,
runnable_type=runnable_type,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

