from typing import Any, Dict, Optional, Union

import httpx

from ...client import Client
from ...models.get_variable_response_200 import GetVariableResponse200
from ...types import UNSET, Response, Unset


def _get_kwargs(
    workspace: str,
    path: str,
    *,
    client: Client,
    decrypt_secret: Union[Unset, None, bool] = UNSET,
) -> Dict[str, Any]:
    url = "{}/w/{workspace}/variables/get/{path}".format(client.base_url, workspace=workspace, path=path)

    headers: Dict[str, str] = client.get_headers()
    cookies: Dict[str, Any] = client.get_cookies()

    params: Dict[str, Any] = {}
    params["decrypt_secret"] = decrypt_secret

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    return {
        "method": "get",
        "url": url,
        "headers": headers,
        "cookies": cookies,
        "timeout": client.get_timeout(),
        "params": params,
    }


def _parse_response(*, response: httpx.Response) -> Optional[GetVariableResponse200]:
    if response.status_code == 200:
        response_200 = GetVariableResponse200.from_dict(response.json())

        return response_200
    return None


def _build_response(*, response: httpx.Response) -> Response[GetVariableResponse200]:
    return Response(
        status_code=response.status_code,
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(response=response),
    )


def sync_detailed(
    workspace: str,
    path: str,
    *,
    client: Client,
    decrypt_secret: Union[Unset, None, bool] = UNSET,
) -> Response[GetVariableResponse200]:
    """get variable

    Args:
        workspace (str):
        path (str):
        decrypt_secret (Union[Unset, None, bool]):

    Returns:
        Response[GetVariableResponse200]
    """

    kwargs = _get_kwargs(
        workspace=workspace,
        path=path,
        client=client,
        decrypt_secret=decrypt_secret,
    )

    response = httpx.request(
        verify=client.verify_ssl,
        **kwargs,
    )

    return _build_response(response=response)


def sync(
    workspace: str,
    path: str,
    *,
    client: Client,
    decrypt_secret: Union[Unset, None, bool] = UNSET,
) -> Optional[GetVariableResponse200]:
    """get variable

    Args:
        workspace (str):
        path (str):
        decrypt_secret (Union[Unset, None, bool]):

    Returns:
        Response[GetVariableResponse200]
    """

    return sync_detailed(
        workspace=workspace,
        path=path,
        client=client,
        decrypt_secret=decrypt_secret,
    ).parsed


async def asyncio_detailed(
    workspace: str,
    path: str,
    *,
    client: Client,
    decrypt_secret: Union[Unset, None, bool] = UNSET,
) -> Response[GetVariableResponse200]:
    """get variable

    Args:
        workspace (str):
        path (str):
        decrypt_secret (Union[Unset, None, bool]):

    Returns:
        Response[GetVariableResponse200]
    """

    kwargs = _get_kwargs(
        workspace=workspace,
        path=path,
        client=client,
        decrypt_secret=decrypt_secret,
    )

    async with httpx.AsyncClient(verify=client.verify_ssl) as _client:
        response = await _client.request(**kwargs)

    return _build_response(response=response)


async def asyncio(
    workspace: str,
    path: str,
    *,
    client: Client,
    decrypt_secret: Union[Unset, None, bool] = UNSET,
) -> Optional[GetVariableResponse200]:
    """get variable

    Args:
        workspace (str):
        path (str):
        decrypt_secret (Union[Unset, None, bool]):

    Returns:
        Response[GetVariableResponse200]
    """

    return (
        await asyncio_detailed(
            workspace=workspace,
            path=path,
            client=client,
            decrypt_secret=decrypt_secret,
        )
    ).parsed
