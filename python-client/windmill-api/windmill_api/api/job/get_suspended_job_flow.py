from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import Union
from ...models.get_suspended_job_flow_response_200 import GetSuspendedJobFlowResponse200
from typing import cast
from typing import Dict
from typing import Optional
from ...types import UNSET, Unset



def _get_kwargs(
    workspace: str,
    id: str,
    resume_id: int,
    signature: str,
    *,
    approver: Union[Unset, None, str] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["approver"] = approver



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    

    

    return {
        "method": "get",
        "url": "/w/{workspace}/jobs_u/get_flow/{id}/{resume_id}/{signature}".format(workspace=workspace,id=id,resume_id=resume_id,signature=signature,),
        "params": params,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[GetSuspendedJobFlowResponse200]:
    if response.status_code == HTTPStatus.OK:
        response_200 = GetSuspendedJobFlowResponse200.from_dict(response.json())



        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[GetSuspendedJobFlowResponse200]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    workspace: str,
    id: str,
    resume_id: int,
    signature: str,
    *,
    client: Union[AuthenticatedClient, Client],
    approver: Union[Unset, None, str] = UNSET,

) -> Response[GetSuspendedJobFlowResponse200]:
    """ get parent flow job of suspended job

    Args:
        workspace (str):
        id (str):
        resume_id (int):
        signature (str):
        approver (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetSuspendedJobFlowResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
id=id,
resume_id=resume_id,
signature=signature,
approver=approver,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    workspace: str,
    id: str,
    resume_id: int,
    signature: str,
    *,
    client: Union[AuthenticatedClient, Client],
    approver: Union[Unset, None, str] = UNSET,

) -> Optional[GetSuspendedJobFlowResponse200]:
    """ get parent flow job of suspended job

    Args:
        workspace (str):
        id (str):
        resume_id (int):
        signature (str):
        approver (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetSuspendedJobFlowResponse200
     """


    return sync_detailed(
        workspace=workspace,
id=id,
resume_id=resume_id,
signature=signature,
client=client,
approver=approver,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    id: str,
    resume_id: int,
    signature: str,
    *,
    client: Union[AuthenticatedClient, Client],
    approver: Union[Unset, None, str] = UNSET,

) -> Response[GetSuspendedJobFlowResponse200]:
    """ get parent flow job of suspended job

    Args:
        workspace (str):
        id (str):
        resume_id (int):
        signature (str):
        approver (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetSuspendedJobFlowResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
id=id,
resume_id=resume_id,
signature=signature,
approver=approver,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    workspace: str,
    id: str,
    resume_id: int,
    signature: str,
    *,
    client: Union[AuthenticatedClient, Client],
    approver: Union[Unset, None, str] = UNSET,

) -> Optional[GetSuspendedJobFlowResponse200]:
    """ get parent flow job of suspended job

    Args:
        workspace (str):
        id (str):
        resume_id (int):
        signature (str):
        approver (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetSuspendedJobFlowResponse200
     """


    return (await asyncio_detailed(
        workspace=workspace,
id=id,
resume_id=resume_id,
signature=signature,
client=client,
approver=approver,

    )).parsed
