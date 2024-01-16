from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import cast
from ...models.s3_resource_info_response_200 import S3ResourceInfoResponse200
from typing import Dict
from ...models.s3_resource_info_json_body import S3ResourceInfoJsonBody



def _get_kwargs(
    workspace: str,
    *,
    json_body: S3ResourceInfoJsonBody,

) -> Dict[str, Any]:
    

    cookies = {}


    

    json_json_body = json_body.to_dict()



    

    return {
        "method": "post",
        "url": "/w/{workspace}/job_helpers/v2/s3_resource_info".format(workspace=workspace,),
        "json": json_json_body,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[S3ResourceInfoResponse200]:
    if response.status_code == HTTPStatus.OK:
        response_200 = S3ResourceInfoResponse200.from_dict(response.json())



        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[S3ResourceInfoResponse200]:
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
    json_body: S3ResourceInfoJsonBody,

) -> Response[S3ResourceInfoResponse200]:
    """ Returns the s3 resource associated to the provided path, or the workspace default S3 resource

    Args:
        workspace (str):
        json_body (S3ResourceInfoJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[S3ResourceInfoResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
json_body=json_body,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: S3ResourceInfoJsonBody,

) -> Optional[S3ResourceInfoResponse200]:
    """ Returns the s3 resource associated to the provided path, or the workspace default S3 resource

    Args:
        workspace (str):
        json_body (S3ResourceInfoJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        S3ResourceInfoResponse200
     """


    return sync_detailed(
        workspace=workspace,
client=client,
json_body=json_body,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: S3ResourceInfoJsonBody,

) -> Response[S3ResourceInfoResponse200]:
    """ Returns the s3 resource associated to the provided path, or the workspace default S3 resource

    Args:
        workspace (str):
        json_body (S3ResourceInfoJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[S3ResourceInfoResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
json_body=json_body,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: S3ResourceInfoJsonBody,

) -> Optional[S3ResourceInfoResponse200]:
    """ Returns the s3 resource associated to the provided path, or the workspace default S3 resource

    Args:
        workspace (str):
        json_body (S3ResourceInfoJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        S3ResourceInfoResponse200
     """


    return (await asyncio_detailed(
        workspace=workspace,
client=client,
json_body=json_body,

    )).parsed
