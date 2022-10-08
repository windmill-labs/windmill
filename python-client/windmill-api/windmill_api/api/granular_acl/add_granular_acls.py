from typing import Any, Dict

import httpx

from ...client import Client
from ...models.add_granular_acls_json_body import AddGranularAclsJsonBody
from ...models.add_granular_acls_kind import AddGranularAclsKind
from ...types import Response


def _get_kwargs(
    workspace: str,
    kind: AddGranularAclsKind,
    path: str,
    *,
    client: Client,
    json_body: AddGranularAclsJsonBody,
) -> Dict[str, Any]:
    url = "{}/w/{workspace}/acls/add/{kind}/{path}".format(client.base_url, workspace=workspace, kind=kind, path=path)

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
    kind: AddGranularAclsKind,
    path: str,
    *,
    client: Client,
    json_body: AddGranularAclsJsonBody,
) -> Response[Any]:
    """add granular acls

    Args:
        workspace (str):
        kind (AddGranularAclsKind):
        path (str):
        json_body (AddGranularAclsJsonBody):

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs(
        workspace=workspace,
        kind=kind,
        path=path,
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
    kind: AddGranularAclsKind,
    path: str,
    *,
    client: Client,
    json_body: AddGranularAclsJsonBody,
) -> Response[Any]:
    """add granular acls

    Args:
        workspace (str):
        kind (AddGranularAclsKind):
        path (str):
        json_body (AddGranularAclsJsonBody):

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs(
        workspace=workspace,
        kind=kind,
        path=path,
        client=client,
        json_body=json_body,
    )

    async with httpx.AsyncClient(verify=client.verify_ssl) as _client:
        response = await _client.request(**kwargs)

    return _build_response(response=response)
