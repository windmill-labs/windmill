from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import cast
from typing import cast, List
from ...models.preview_schedule_json_body import PreviewScheduleJsonBody
from typing import Dict
import datetime
from dateutil.parser import isoparse



def _get_kwargs(
    *,
    json_body: PreviewScheduleJsonBody,

) -> Dict[str, Any]:
    

    cookies = {}


    

    json_json_body = json_body.to_dict()



    

    return {
        "method": "post",
        "url": "/schedules/preview",
        "json": json_json_body,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[List[datetime.datetime]]:
    if response.status_code == HTTPStatus.OK:
        response_200 = []
        _response_200 = response.json()
        for response_200_item_data in (_response_200):
            response_200_item = isoparse(response_200_item_data)



            response_200.append(response_200_item)

        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[List[datetime.datetime]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: PreviewScheduleJsonBody,

) -> Response[List[datetime.datetime]]:
    """ preview schedule

    Args:
        json_body (PreviewScheduleJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[List[datetime.datetime]]
     """


    kwargs = _get_kwargs(
        json_body=json_body,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: PreviewScheduleJsonBody,

) -> Optional[List[datetime.datetime]]:
    """ preview schedule

    Args:
        json_body (PreviewScheduleJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        List[datetime.datetime]
     """


    return sync_detailed(
        client=client,
json_body=json_body,

    ).parsed

async def asyncio_detailed(
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: PreviewScheduleJsonBody,

) -> Response[List[datetime.datetime]]:
    """ preview schedule

    Args:
        json_body (PreviewScheduleJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[List[datetime.datetime]]
     """


    kwargs = _get_kwargs(
        json_body=json_body,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    *,
    client: Union[AuthenticatedClient, Client],
    json_body: PreviewScheduleJsonBody,

) -> Optional[List[datetime.datetime]]:
    """ preview schedule

    Args:
        json_body (PreviewScheduleJsonBody):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        List[datetime.datetime]
     """


    return (await asyncio_detailed(
        client=client,
json_body=json_body,

    )).parsed
