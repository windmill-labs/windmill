from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import cast
from ...models.get_job_response_200 import GetJobResponse200
from typing import Dict



def _get_kwargs(
    workspace: str,
    id: str,

) -> Dict[str, Any]:
    

    cookies = {}


    

    

    

    return {
        "method": "get",
        "url": "/w/{workspace}/jobs_u/get/{id}".format(workspace=workspace,id=id,),
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[GetJobResponse200]:
    if response.status_code == HTTPStatus.OK:
        response_200 = GetJobResponse200.from_dict(response.json())



        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[GetJobResponse200]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    workspace: str,
    id: str,
    *,
    client: Union[AuthenticatedClient, Client],

) -> Response[GetJobResponse200]:
    """ get job

    Args:
        workspace (str):
        id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetJobResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
id=id,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    workspace: str,
    id: str,
    *,
    client: Union[AuthenticatedClient, Client],

) -> Optional[GetJobResponse200]:
    """ get job

    Args:
        workspace (str):
        id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetJobResponse200
     """


    return sync_detailed(
        workspace=workspace,
id=id,
client=client,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    id: str,
    *,
    client: Union[AuthenticatedClient, Client],

) -> Response[GetJobResponse200]:
    """ get job

    Args:
        workspace (str):
        id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetJobResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
id=id,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    workspace: str,
    id: str,
    *,
    client: Union[AuthenticatedClient, Client],

) -> Optional[GetJobResponse200]:
    """ get job

    Args:
        workspace (str):
        id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetJobResponse200
     """


    return (await asyncio_detailed(
        workspace=workspace,
id=id,
client=client,

    )).parsed
