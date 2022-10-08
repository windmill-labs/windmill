import datetime
from typing import Any, Dict, List, Optional

import httpx
from dateutil.parser import isoparse

from ...client import Client
from ...models.preview_schedule_json_body import PreviewScheduleJsonBody
from ...types import Response


def _get_kwargs(
    *,
    client: Client,
    json_body: PreviewScheduleJsonBody,
) -> Dict[str, Any]:
    url = "{}/schedules/preview".format(client.base_url)

    headers: Dict[str, str] = client.get_headers()
    cookies: Dict[str, Any] = client.get_cookies()

    json_json_body = json_body.to_dict()

    return {
        "method": "post",
        "url": url,
        "headers": headers,
        "cookies": cookies,
        "timeout": client.get_timeout(),
        "json": json_json_body,
    }


def _parse_response(*, response: httpx.Response) -> Optional[List[datetime.datetime]]:
    if response.status_code == 200:
        response_200 = []
        _response_200 = response.json()
        for response_200_item_data in _response_200:
            response_200_item = isoparse(response_200_item_data)

            response_200.append(response_200_item)

        return response_200
    return None


def _build_response(*, response: httpx.Response) -> Response[List[datetime.datetime]]:
    return Response(
        status_code=response.status_code,
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(response=response),
    )


def sync_detailed(
    *,
    client: Client,
    json_body: PreviewScheduleJsonBody,
) -> Response[List[datetime.datetime]]:
    """preview schedule

    Args:
        json_body (PreviewScheduleJsonBody):

    Returns:
        Response[List[datetime.datetime]]
    """

    kwargs = _get_kwargs(
        client=client,
        json_body=json_body,
    )

    response = httpx.request(
        verify=client.verify_ssl,
        **kwargs,
    )

    return _build_response(response=response)


def sync(
    *,
    client: Client,
    json_body: PreviewScheduleJsonBody,
) -> Optional[List[datetime.datetime]]:
    """preview schedule

    Args:
        json_body (PreviewScheduleJsonBody):

    Returns:
        Response[List[datetime.datetime]]
    """

    return sync_detailed(
        client=client,
        json_body=json_body,
    ).parsed


async def asyncio_detailed(
    *,
    client: Client,
    json_body: PreviewScheduleJsonBody,
) -> Response[List[datetime.datetime]]:
    """preview schedule

    Args:
        json_body (PreviewScheduleJsonBody):

    Returns:
        Response[List[datetime.datetime]]
    """

    kwargs = _get_kwargs(
        client=client,
        json_body=json_body,
    )

    async with httpx.AsyncClient(verify=client.verify_ssl) as _client:
        response = await _client.request(**kwargs)

    return _build_response(response=response)


async def asyncio(
    *,
    client: Client,
    json_body: PreviewScheduleJsonBody,
) -> Optional[List[datetime.datetime]]:
    """preview schedule

    Args:
        json_body (PreviewScheduleJsonBody):

    Returns:
        Response[List[datetime.datetime]]
    """

    return (
        await asyncio_detailed(
            client=client,
            json_body=json_body,
        )
    ).parsed
