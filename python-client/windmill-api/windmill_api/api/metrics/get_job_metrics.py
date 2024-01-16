from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import cast
from typing import Dict
from ...models.get_job_metrics_json_body import GetJobMetricsJsonBody
from ...models.get_job_metrics_response_200 import GetJobMetricsResponse200



def _get_kwargs(
    workspace: str,
    id: str,
    *,
    json_body: GetJobMetricsJsonBody,

) -> Dict[str, Any]:
    

    cookies = {}


    

    json_json_body = json_body.to_dict()



    

    return {
        "method": "post",
        "url": "/w/{workspace}/job_metrics/get/{id}".format(workspace=workspace,id=id,),
        "json": json_json_body,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[GetJobMetricsResponse200]:
    if response.status_code == HTTPStatus.OK:
        response_200 = GetJobMetricsResponse200.from_dict(response.json())



        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[GetJobMetricsResponse200]:
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
    json_body: GetJobMetricsJsonBody,

) -> Response[GetJobMetricsResponse200]:
    """ get job metrics

    Args:
        workspace (str):
        id (str):
        json_body (GetJobMetricsJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetJobMetricsResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
id=id,
json_body=json_body,

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
    json_body: GetJobMetricsJsonBody,

) -> Optional[GetJobMetricsResponse200]:
    """ get job metrics

    Args:
        workspace (str):
        id (str):
        json_body (GetJobMetricsJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetJobMetricsResponse200
     """


    return sync_detailed(
        workspace=workspace,
id=id,
client=client,
json_body=json_body,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    id: str,
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: GetJobMetricsJsonBody,

) -> Response[GetJobMetricsResponse200]:
    """ get job metrics

    Args:
        workspace (str):
        id (str):
        json_body (GetJobMetricsJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[GetJobMetricsResponse200]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
id=id,
json_body=json_body,

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
    json_body: GetJobMetricsJsonBody,

) -> Optional[GetJobMetricsResponse200]:
    """ get job metrics

    Args:
        workspace (str):
        id (str):
        json_body (GetJobMetricsJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        GetJobMetricsResponse200
     """


    return (await asyncio_detailed(
        workspace=workspace,
id=id,
client=client,
json_body=json_body,

    )).parsed
