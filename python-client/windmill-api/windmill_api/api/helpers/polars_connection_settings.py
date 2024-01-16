from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import cast
from typing import Dict
from ...models.polars_connection_settings_json_body import PolarsConnectionSettingsJsonBody
from ...models.polars_connection_settings_response_200 import PolarsConnectionSettingsResponse200



def _get_kwargs(
    workspace: str,
    *,
    json_body: PolarsConnectionSettingsJsonBody,

) -> Dict[str, Any]:
    

    cookies = {}


    

    json_json_body = json_body.to_dict()



    

    return {
        "method": "post",
        "url": "/w/{workspace}/job_helpers/polars_connection_settings".format(workspace=workspace,),
        "json": json_json_body,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[PolarsConnectionSettingsResponse200]:
    if response.status_code == HTTPStatus.OK:
        response_200 = PolarsConnectionSettingsResponse200.from_dict(response.json())



        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[PolarsConnectionSettingsResponse200]:
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
    json_body: PolarsConnectionSettingsJsonBody,

) -> Response[PolarsConnectionSettingsResponse200]:
    """ Converts an S3 resource to the set of arguments necessary to connect Polars to an S3 bucket

    Args:
        workspace (str):
        json_body (PolarsConnectionSettingsJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[PolarsConnectionSettingsResponse200]
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
    json_body: PolarsConnectionSettingsJsonBody,

) -> Optional[PolarsConnectionSettingsResponse200]:
    """ Converts an S3 resource to the set of arguments necessary to connect Polars to an S3 bucket

    Args:
        workspace (str):
        json_body (PolarsConnectionSettingsJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        PolarsConnectionSettingsResponse200
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
    json_body: PolarsConnectionSettingsJsonBody,

) -> Response[PolarsConnectionSettingsResponse200]:
    """ Converts an S3 resource to the set of arguments necessary to connect Polars to an S3 bucket

    Args:
        workspace (str):
        json_body (PolarsConnectionSettingsJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[PolarsConnectionSettingsResponse200]
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
    json_body: PolarsConnectionSettingsJsonBody,

) -> Optional[PolarsConnectionSettingsResponse200]:
    """ Converts an S3 resource to the set of arguments necessary to connect Polars to an S3 bucket

    Args:
        workspace (str):
        json_body (PolarsConnectionSettingsJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        PolarsConnectionSettingsResponse200
     """


    return (await asyncio_detailed(
        workspace=workspace,
client=client,
json_body=json_body,

    )).parsed
