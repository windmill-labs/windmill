from typing import Any, Dict

import httpx

from ...client import Client
from ...models.resume_suspended_job_json_body import ResumeSuspendedJobJsonBody
from ...types import Response


def _get_kwargs(
    workspace: str,
    id: str,
    *,
    client: Client,
    json_body: ResumeSuspendedJobJsonBody,
) -> Dict[str, Any]:
    url = "{}/w/{workspace}/jobs/resume/{id}".format(client.base_url, workspace=workspace, id=id)

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


def _build_response(*, response: httpx.Response) -> Response[Any]:
    return Response(
        status_code=response.status_code,
        content=response.content,
        headers=response.headers,
        parsed=None,
    )


def sync_detailed(
    workspace: str,
    id: str,
    *,
    client: Client,
    json_body: ResumeSuspendedJobJsonBody,
) -> Response[Any]:
    """resume a job for a suspended flow

    Args:
        workspace (str):
        id (str):
        json_body (ResumeSuspendedJobJsonBody):

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs(
        workspace=workspace,
        id=id,
        client=client,
        json_body=json_body,
    )

    response = httpx.request(
        verify=client.verify_ssl,
        **kwargs,
    )

    return _build_response(response=response)


async def asyncio_detailed(
    workspace: str,
    id: str,
    *,
    client: Client,
    json_body: ResumeSuspendedJobJsonBody,
) -> Response[Any]:
    """resume a job for a suspended flow

    Args:
        workspace (str):
        id (str):
        json_body (ResumeSuspendedJobJsonBody):

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs(
        workspace=workspace,
        id=id,
        client=client,
        json_body=json_body,
    )

    async with httpx.AsyncClient(verify=client.verify_ssl) as _client:
        response = await _client.request(**kwargs)

    return _build_response(response=response)
