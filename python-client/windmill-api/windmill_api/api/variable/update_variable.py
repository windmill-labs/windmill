from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import Union
from typing import cast
from typing import Dict
from ...models.update_variable_json_body import UpdateVariableJsonBody
from typing import Optional
from ...types import UNSET, Unset



def _get_kwargs(
    workspace: str,
    path: str,
    *,
    json_body: UpdateVariableJsonBody,
    already_encrypted: Union[Unset, None, bool] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["already_encrypted"] = already_encrypted



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    json_json_body = json_body.to_dict()



    

    return {
        "method": "post",
        "url": "/w/{workspace}/variables/update/{path}".format(workspace=workspace,path=path,),
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
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: UpdateVariableJsonBody,
    already_encrypted: Union[Unset, None, bool] = UNSET,

) -> Response[Any]:
    """ update variable

    Args:
        workspace (str):
        path (str):
        already_encrypted (Union[Unset, None, bool]):
        json_body (UpdateVariableJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
path=path,
json_body=json_body,
already_encrypted=already_encrypted,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


async def asyncio_detailed(
    workspace: str,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: UpdateVariableJsonBody,
    already_encrypted: Union[Unset, None, bool] = UNSET,

) -> Response[Any]:
    """ update variable

    Args:
        workspace (str):
        path (str):
        already_encrypted (Union[Unset, None, bool]):
        json_body (UpdateVariableJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
path=path,
json_body=json_body,
already_encrypted=already_encrypted,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

