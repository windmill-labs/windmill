from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import Union
from typing import cast
from typing import Dict
from ...models.resume_suspended_job_post_json_body import ResumeSuspendedJobPostJsonBody
from typing import Optional
from ...types import UNSET, Unset



def _get_kwargs(
    workspace: str,
    id: str,
    resume_id: int,
    signature: str,
    *,
    json_body: ResumeSuspendedJobPostJsonBody,
    approver: Union[Unset, None, str] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["approver"] = approver



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    json_json_body = json_body.to_dict()



    

    return {
        "method": "post",
        "url": "/w/{workspace}/jobs_u/resume/{id}/{resume_id}/{signature}".format(workspace=workspace,id=id,resume_id=resume_id,signature=signature,),
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
    id: str,
    resume_id: int,
    signature: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: ResumeSuspendedJobPostJsonBody,
    approver: Union[Unset, None, str] = UNSET,

) -> Response[Any]:
    """ resume a job for a suspended flow

    Args:
        workspace (str):
        id (str):
        resume_id (int):
        signature (str):
        approver (Union[Unset, None, str]):
        json_body (ResumeSuspendedJobPostJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
id=id,
resume_id=resume_id,
signature=signature,
json_body=json_body,
approver=approver,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


async def asyncio_detailed(
    workspace: str,
    id: str,
    resume_id: int,
    signature: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: ResumeSuspendedJobPostJsonBody,
    approver: Union[Unset, None, str] = UNSET,

) -> Response[Any]:
    """ resume a job for a suspended flow

    Args:
        workspace (str):
        id (str):
        resume_id (int):
        signature (str):
        approver (Union[Unset, None, str]):
        json_body (ResumeSuspendedJobPostJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
id=id,
resume_id=resume_id,
signature=signature,
json_body=json_body,
approver=approver,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

