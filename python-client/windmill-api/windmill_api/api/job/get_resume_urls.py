from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import Union
from typing import cast
from typing import Dict
from ...models.get_resume_urls_response_200 import GetResumeUrlsResponse200
from typing import Optional
from ...types import UNSET, Unset



def _get_kwargs(
    workspace: str,
    id: str,
    resume_id: int,
    *,
    approver: Union[Unset, None, str] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["approver"] = approver



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    

    

    return {
        "method": "get",
        "url": "/w/{workspace}/jobs/resume_urls/{id}/{resume_id}".format(workspace=workspace,id=id,resume_id=resume_id,),
        "params": params,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[GetResumeUrlsResponse200]:
    if response.status_code == HTTPStatus.OK:
        response_200 = GetResumeUrlsResponse200.from_dict(response.json())



        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[GetResumeUrlsResponse200]:
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
    *,
    client: Union[AuthenticatedClient, Client],
    approver: Union[Unset, None, str] = UNSET,

) -> Response[GetResumeUrlsResponse200]:
    """ get resume urls given a job_id, resume_id and a nonce to resume a flow

    Args:
        workspace (str):
        id (str):
        resume_id (int):
        approver (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetResumeUrlsResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
id=id,
resume_id=resume_id,
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
    *,
    client: Union[AuthenticatedClient, Client],
    approver: Union[Unset, None, str] = UNSET,

) -> Optional[GetResumeUrlsResponse200]:
    """ get resume urls given a job_id, resume_id and a nonce to resume a flow

    Args:
        workspace (str):
        id (str):
        resume_id (int):
        approver (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetResumeUrlsResponse200
     """


    return sync_detailed(
        workspace=workspace,
id=id,
resume_id=resume_id,
client=client,
approver=approver,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    id: str,
    resume_id: int,
    *,
    client: Union[AuthenticatedClient, Client],
    approver: Union[Unset, None, str] = UNSET,

) -> Response[GetResumeUrlsResponse200]:
    """ get resume urls given a job_id, resume_id and a nonce to resume a flow

    Args:
        workspace (str):
        id (str):
        resume_id (int):
        approver (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetResumeUrlsResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
id=id,
resume_id=resume_id,
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
    *,
    client: Union[AuthenticatedClient, Client],
    approver: Union[Unset, None, str] = UNSET,

) -> Optional[GetResumeUrlsResponse200]:
    """ get resume urls given a job_id, resume_id and a nonce to resume a flow

    Args:
        workspace (str):
        id (str):
        resume_id (int):
        approver (Union[Unset, None, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetResumeUrlsResponse200
     """


    return (await asyncio_detailed(
        workspace=workspace,
id=id,
resume_id=resume_id,
client=client,
approver=approver,

    )).parsed
