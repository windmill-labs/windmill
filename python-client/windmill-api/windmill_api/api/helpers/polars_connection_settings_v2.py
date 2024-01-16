from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import cast
from typing import Dict
from ...models.polars_connection_settings_v2_json_body import PolarsConnectionSettingsV2JsonBody
from ...models.polars_connection_settings_v2_response_200 import PolarsConnectionSettingsV2Response200



def _get_kwargs(
    workspace: str,
    *,
    json_body: PolarsConnectionSettingsV2JsonBody,

) -> Dict[str, Any]:
    

    cookies = {}


    

    json_json_body = json_body.to_dict()



    

    return {
        "method": "post",
        "url": "/w/{workspace}/job_helpers/v2/polars_connection_settings".format(workspace=workspace,),
        "json": json_json_body,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[PolarsConnectionSettingsV2Response200]:
    if response.status_code == HTTPStatus.OK:
        response_200 = PolarsConnectionSettingsV2Response200.from_dict(response.json())



        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[PolarsConnectionSettingsV2Response200]:
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
    json_body: PolarsConnectionSettingsV2JsonBody,

) -> Response[PolarsConnectionSettingsV2Response200]:
    """ Converts an S3 resource to the set of arguments necessary to connect Polars to an S3 bucket

    Args:
        workspace (str):
        json_body (PolarsConnectionSettingsV2JsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[PolarsConnectionSettingsV2Response200]
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
    json_body: PolarsConnectionSettingsV2JsonBody,

) -> Optional[PolarsConnectionSettingsV2Response200]:
    """ Converts an S3 resource to the set of arguments necessary to connect Polars to an S3 bucket

    Args:
        workspace (str):
        json_body (PolarsConnectionSettingsV2JsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        PolarsConnectionSettingsV2Response200
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
    json_body: PolarsConnectionSettingsV2JsonBody,

) -> Response[PolarsConnectionSettingsV2Response200]:
    """ Converts an S3 resource to the set of arguments necessary to connect Polars to an S3 bucket

    Args:
        workspace (str):
        json_body (PolarsConnectionSettingsV2JsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[PolarsConnectionSettingsV2Response200]
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
    json_body: PolarsConnectionSettingsV2JsonBody,

) -> Optional[PolarsConnectionSettingsV2Response200]:
    """ Converts an S3 resource to the set of arguments necessary to connect Polars to an S3 bucket

    Args:
        workspace (str):
        json_body (PolarsConnectionSettingsV2JsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        PolarsConnectionSettingsV2Response200
     """


    return (await asyncio_detailed(
        workspace=workspace,
client=client,
json_body=json_body,

    )).parsed
