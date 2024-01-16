from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from ...models.remove_granular_acls_json_body import RemoveGranularAclsJsonBody
from typing import cast
from ...models.remove_granular_acls_kind import RemoveGranularAclsKind
from typing import Dict



def _get_kwargs(
    workspace: str,
    kind: RemoveGranularAclsKind,
    path: str,
    *,
    json_body: RemoveGranularAclsJsonBody,

) -> Dict[str, Any]:
    

    cookies = {}


    

    json_json_body = json_body.to_dict()



    

    return {
        "method": "post",
        "url": "/w/{workspace}/acls/remove/{kind}/{path}".format(workspace=workspace,kind=kind,path=path,),
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
    kind: RemoveGranularAclsKind,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: RemoveGranularAclsJsonBody,

) -> Response[Any]:
    """ remove granular acls

    Args:
        workspace (str):
        kind (RemoveGranularAclsKind):
        path (str):
        json_body (RemoveGranularAclsJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
kind=kind,
path=path,
json_body=json_body,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


async def asyncio_detailed(
    workspace: str,
    kind: RemoveGranularAclsKind,
    path: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: RemoveGranularAclsJsonBody,

) -> Response[Any]:
    """ remove granular acls

    Args:
        workspace (str):
        kind (RemoveGranularAclsKind):
        path (str):
        json_body (RemoveGranularAclsJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
kind=kind,
path=path,
json_body=json_body,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

