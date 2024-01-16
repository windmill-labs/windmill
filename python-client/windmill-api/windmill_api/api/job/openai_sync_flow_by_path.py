from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import Union
from typing import cast
from ...models.openai_sync_flow_by_path_json_body import OpenaiSyncFlowByPathJsonBody
from typing import Dict
from typing import Optional
from ...types import UNSET, Unset



def _get_kwargs(
    workspace: str,
    path: str,
    *,
    json_body: OpenaiSyncFlowByPathJsonBody,
    include_header: Union[Unset, None, str] = UNSET,
    queue_limit: Union[Unset, None, str] = UNSET,
    job_id: Union[Unset, None, str] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["include_header"] = include_header


    params["queue_limit"] = queue_limit


    params["job_id"] = job_id



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    json_json_body = json_body.to_dict()



    

    return {
        "method": "post",
        "url": "/w/{workspace}/jobs/openai_sync/f/{path}".format(workspace=workspace,path=path,),
        "json": json_json_body,
        "params": params,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[Any]:
    if response.status_code == HTTPStatus.OK:
        return None
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
    json_body: OpenaiSyncFlowByPathJsonBody,
    include_header: Union[Unset, None, str] = UNSET,
    queue_limit: Union[Unset, None, str] = UNSET,
    job_id: Union[Unset, None, str] = UNSET,

) -> Response[Any]:
    """ run flow by path and wait until completion in openai format

    Args:
        workspace (str):
        path (str):
        include_header (Union[Unset, None, str]):
        queue_limit (Union[Unset, None, str]):
        job_id (Union[Unset, None, str]):
        json_body (OpenaiSyncFlowByPathJsonBody):

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
include_header=include_header,
queue_limit=queue_limit,
job_id=job_id,

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
    json_body: OpenaiSyncFlowByPathJsonBody,
    include_header: Union[Unset, None, str] = UNSET,
    queue_limit: Union[Unset, None, str] = UNSET,
    job_id: Union[Unset, None, str] = UNSET,

) -> Response[Any]:
    """ run flow by path and wait until completion in openai format

    Args:
        workspace (str):
        path (str):
        include_header (Union[Unset, None, str]):
        queue_limit (Union[Unset, None, str]):
        job_id (Union[Unset, None, str]):
        json_body (OpenaiSyncFlowByPathJsonBody):

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
include_header=include_header,
queue_limit=queue_limit,
job_id=job_id,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

