from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import cast
from ...models.update_script_history_json_body import UpdateScriptHistoryJsonBody
from typing import Dict



def _get_kwargs(
    workspace: str,
    hash_: str,
    path: str,
    *,
    json_body: UpdateScriptHistoryJsonBody,

) -> Dict[str, Any]:
    

    cookies = {}


    

    json_json_body = json_body.to_dict()



    

    return {
        "method": "post",
        "url": "/w/{workspace}/scripts/history_update/h/{hash}/p/{path}".format(workspace=workspace,hash=hash_,path=path,),
        "json": json_json_body,
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
    hash_: str,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: UpdateScriptHistoryJsonBody,

) -> Response[Any]:
    """ update history of a script

    Args:
        workspace (str):
        hash_ (str):
        path (str):
        json_body (UpdateScriptHistoryJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
hash_=hash_,
path=path,
json_body=json_body,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


async def asyncio_detailed(
    workspace: str,
    hash_: str,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: UpdateScriptHistoryJsonBody,

) -> Response[Any]:
    """ update history of a script

    Args:
        workspace (str):
        hash_ (str):
        path (str):
        json_body (UpdateScriptHistoryJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
hash_=hash_,
path=path,
json_body=json_body,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

