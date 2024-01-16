from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import cast
from ...models.duckdb_connection_settings_v2_json_body import DuckdbConnectionSettingsV2JsonBody
from typing import Dict
from ...models.duckdb_connection_settings_v2_response_200 import DuckdbConnectionSettingsV2Response200



def _get_kwargs(
    workspace: str,
    *,
    json_body: DuckdbConnectionSettingsV2JsonBody,

) -> Dict[str, Any]:
    

    cookies = {}


    

    json_json_body = json_body.to_dict()



    

    return {
        "method": "post",
        "url": "/w/{workspace}/job_helpers/v2/duckdb_connection_settings".format(workspace=workspace,),
        "json": json_json_body,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[DuckdbConnectionSettingsV2Response200]:
    if response.status_code == HTTPStatus.OK:
        response_200 = DuckdbConnectionSettingsV2Response200.from_dict(response.json())



        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[DuckdbConnectionSettingsV2Response200]:
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
    json_body: DuckdbConnectionSettingsV2JsonBody,

) -> Response[DuckdbConnectionSettingsV2Response200]:
    """ Converts an S3 resource to the set of instructions necessary to connect DuckDB to an S3 bucket

    Args:
        workspace (str):
        json_body (DuckdbConnectionSettingsV2JsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[DuckdbConnectionSettingsV2Response200]
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
    json_body: DuckdbConnectionSettingsV2JsonBody,

) -> Optional[DuckdbConnectionSettingsV2Response200]:
    """ Converts an S3 resource to the set of instructions necessary to connect DuckDB to an S3 bucket

    Args:
        workspace (str):
        json_body (DuckdbConnectionSettingsV2JsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        DuckdbConnectionSettingsV2Response200
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
    json_body: DuckdbConnectionSettingsV2JsonBody,

) -> Response[DuckdbConnectionSettingsV2Response200]:
    """ Converts an S3 resource to the set of instructions necessary to connect DuckDB to an S3 bucket

    Args:
        workspace (str):
        json_body (DuckdbConnectionSettingsV2JsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[DuckdbConnectionSettingsV2Response200]
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
    json_body: DuckdbConnectionSettingsV2JsonBody,

) -> Optional[DuckdbConnectionSettingsV2Response200]:
    """ Converts an S3 resource to the set of instructions necessary to connect DuckDB to an S3 bucket

    Args:
        workspace (str):
        json_body (DuckdbConnectionSettingsV2JsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        DuckdbConnectionSettingsV2Response200
     """


    return (await asyncio_detailed(
        workspace=workspace,
client=client,
json_body=json_body,

    )).parsed
