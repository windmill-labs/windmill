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
from ...models.list_audit_logs_action_kind import ListAuditLogsActionKind
import datetime
from dateutil.parser import isoparse
from ...models.list_audit_logs_response_200_item import ListAuditLogsResponse200Item
from typing import Optional
from ...types import UNSET, Unset



def _get_kwargs(
    workspace: str,
    *,
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
    before: Union[Unset, None, datetime.datetime] = UNSET,
    after: Union[Unset, None, datetime.datetime] = UNSET,
    username: Union[Unset, None, str] = UNSET,
    operation: Union[Unset, None, str] = UNSET,
    resource: Union[Unset, None, str] = UNSET,
    action_kind: Union[Unset, None, ListAuditLogsActionKind] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["page"] = page


    params["per_page"] = per_page


    json_before: Union[Unset, None, str] = UNSET
    if not isinstance(before, Unset):
        json_before = before.isoformat() if before else None

    params["before"] = json_before


    json_after: Union[Unset, None, str] = UNSET
    if not isinstance(after, Unset):
        json_after = after.isoformat() if after else None

    params["after"] = json_after


    params["username"] = username


    params["operation"] = operation


    params["resource"] = resource


    json_action_kind: Union[Unset, None, str] = UNSET
    if not isinstance(action_kind, Unset):
        json_action_kind = action_kind.value if action_kind else None

    params["action_kind"] = json_action_kind



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    

    

    return {
        "method": "get",
        "url": "/w/{workspace}/audit/list".format(workspace=workspace,),
        "params": params,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[List['ListAuditLogsResponse200Item']]:
    if response.status_code == HTTPStatus.OK:
        response_200 = []
        _response_200 = response.json()
        for response_200_item_data in (_response_200):
            response_200_item = ListAuditLogsResponse200Item.from_dict(response_200_item_data)



            response_200.append(response_200_item)

        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[List['ListAuditLogsResponse200Item']]:
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
    before: Union[Unset, None, datetime.datetime] = UNSET,
    after: Union[Unset, None, datetime.datetime] = UNSET,
    username: Union[Unset, None, str] = UNSET,
    operation: Union[Unset, None, str] = UNSET,
    resource: Union[Unset, None, str] = UNSET,
    action_kind: Union[Unset, None, ListAuditLogsActionKind] = UNSET,

) -> Response[List['ListAuditLogsResponse200Item']]:
    """ list audit logs (requires admin privilege)

    Args:
        workspace (str):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):
        before (Union[Unset, None, datetime.datetime]):
        after (Union[Unset, None, datetime.datetime]):
        username (Union[Unset, None, str]):
        operation (Union[Unset, None, str]):
        resource (Union[Unset, None, str]):
        action_kind (Union[Unset, None, ListAuditLogsActionKind]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[List['ListAuditLogsResponse200Item']]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
page=page,
per_page=per_page,
before=before,
after=after,
username=username,
operation=operation,
resource=resource,
action_kind=action_kind,

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
    before: Union[Unset, None, datetime.datetime] = UNSET,
    after: Union[Unset, None, datetime.datetime] = UNSET,
    username: Union[Unset, None, str] = UNSET,
    operation: Union[Unset, None, str] = UNSET,
    resource: Union[Unset, None, str] = UNSET,
    action_kind: Union[Unset, None, ListAuditLogsActionKind] = UNSET,

) -> Optional[List['ListAuditLogsResponse200Item']]:
    """ list audit logs (requires admin privilege)

    Args:
        workspace (str):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):
        before (Union[Unset, None, datetime.datetime]):
        after (Union[Unset, None, datetime.datetime]):
        username (Union[Unset, None, str]):
        operation (Union[Unset, None, str]):
        resource (Union[Unset, None, str]):
        action_kind (Union[Unset, None, ListAuditLogsActionKind]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        List['ListAuditLogsResponse200Item']
     """


    return sync_detailed(
        workspace=workspace,
client=client,
page=page,
per_page=per_page,
before=before,
after=after,
username=username,
operation=operation,
resource=resource,
action_kind=action_kind,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    page: Union[Unset, None, int] = UNSET,
    per_page: Union[Unset, None, int] = UNSET,
    before: Union[Unset, None, datetime.datetime] = UNSET,
    after: Union[Unset, None, datetime.datetime] = UNSET,
    username: Union[Unset, None, str] = UNSET,
    operation: Union[Unset, None, str] = UNSET,
    resource: Union[Unset, None, str] = UNSET,
    action_kind: Union[Unset, None, ListAuditLogsActionKind] = UNSET,

) -> Response[List['ListAuditLogsResponse200Item']]:
    """ list audit logs (requires admin privilege)

    Args:
        workspace (str):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):
        before (Union[Unset, None, datetime.datetime]):
        after (Union[Unset, None, datetime.datetime]):
        username (Union[Unset, None, str]):
        operation (Union[Unset, None, str]):
        resource (Union[Unset, None, str]):
        action_kind (Union[Unset, None, ListAuditLogsActionKind]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[List['ListAuditLogsResponse200Item']]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
page=page,
per_page=per_page,
before=before,
after=after,
username=username,
operation=operation,
resource=resource,
action_kind=action_kind,

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
    before: Union[Unset, None, datetime.datetime] = UNSET,
    after: Union[Unset, None, datetime.datetime] = UNSET,
    username: Union[Unset, None, str] = UNSET,
    operation: Union[Unset, None, str] = UNSET,
    resource: Union[Unset, None, str] = UNSET,
    action_kind: Union[Unset, None, ListAuditLogsActionKind] = UNSET,

) -> Optional[List['ListAuditLogsResponse200Item']]:
    """ list audit logs (requires admin privilege)

    Args:
        workspace (str):
        page (Union[Unset, None, int]):
        per_page (Union[Unset, None, int]):
        before (Union[Unset, None, datetime.datetime]):
        after (Union[Unset, None, datetime.datetime]):
        username (Union[Unset, None, str]):
        operation (Union[Unset, None, str]):
        resource (Union[Unset, None, str]):
        action_kind (Union[Unset, None, ListAuditLogsActionKind]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        List['ListAuditLogsResponse200Item']
     """


    return (await asyncio_detailed(
        workspace=workspace,
client=client,
page=page,
per_page=per_page,
before=before,
after=after,
username=username,
operation=operation,
resource=resource,
action_kind=action_kind,

    )).parsed
