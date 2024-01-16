from http import HTTPStatus
from typing import Any, Dict, List, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from typing import Union
from typing import cast
from typing import cast, List
from typing import Dict
from ...models.list_jobs_response_200_item import ListJobsResponse200Item
import datetime
from dateutil.parser import isoparse
from typing import Optional
from ...types import UNSET, Unset



def _get_kwargs(
    workspace: str,
    *,
    created_by: Union[Unset, None, str] = UNSET,
    parent_job: Union[Unset, None, str] = UNSET,
    script_path_exact: Union[Unset, None, str] = UNSET,
    script_path_start: Union[Unset, None, str] = UNSET,
    schedule_path: Union[Unset, None, str] = UNSET,
    script_hash: Union[Unset, None, str] = UNSET,
    started_before: Union[Unset, None, datetime.datetime] = UNSET,
    started_after: Union[Unset, None, datetime.datetime] = UNSET,
    created_or_started_before: Union[Unset, None, datetime.datetime] = UNSET,
    running: Union[Unset, None, bool] = UNSET,
    scheduled_for_before_now: Union[Unset, None, bool] = UNSET,
    created_or_started_after: Union[Unset, None, datetime.datetime] = UNSET,
    job_kinds: Union[Unset, None, str] = UNSET,
    args: Union[Unset, None, str] = UNSET,
    tag: Union[Unset, None, str] = UNSET,
    result: Union[Unset, None, str] = UNSET,
    is_skipped: Union[Unset, None, bool] = UNSET,
    is_flow_step: Union[Unset, None, bool] = UNSET,
    success: Union[Unset, None, bool] = UNSET,

) -> Dict[str, Any]:
    

    cookies = {}


    params: Dict[str, Any] = {}
    params["created_by"] = created_by


    params["parent_job"] = parent_job


    params["script_path_exact"] = script_path_exact


    params["script_path_start"] = script_path_start


    params["schedule_path"] = schedule_path


    params["script_hash"] = script_hash


    json_started_before: Union[Unset, None, str] = UNSET
    if not isinstance(started_before, Unset):
        json_started_before = started_before.isoformat() if started_before else None

    params["started_before"] = json_started_before


    json_started_after: Union[Unset, None, str] = UNSET
    if not isinstance(started_after, Unset):
        json_started_after = started_after.isoformat() if started_after else None

    params["started_after"] = json_started_after


    json_created_or_started_before: Union[Unset, None, str] = UNSET
    if not isinstance(created_or_started_before, Unset):
        json_created_or_started_before = created_or_started_before.isoformat() if created_or_started_before else None

    params["created_or_started_before"] = json_created_or_started_before


    params["running"] = running


    params["scheduled_for_before_now"] = scheduled_for_before_now


    json_created_or_started_after: Union[Unset, None, str] = UNSET
    if not isinstance(created_or_started_after, Unset):
        json_created_or_started_after = created_or_started_after.isoformat() if created_or_started_after else None

    params["created_or_started_after"] = json_created_or_started_after


    params["job_kinds"] = job_kinds


    params["args"] = args


    params["tag"] = tag


    params["result"] = result


    params["is_skipped"] = is_skipped


    params["is_flow_step"] = is_flow_step


    params["success"] = success



    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}


    

    

    return {
        "method": "get",
        "url": "/w/{workspace}/jobs/list".format(workspace=workspace,),
        "params": params,
    }


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[List['ListJobsResponse200Item']]:
    if response.status_code == HTTPStatus.OK:
        response_200 = []
        _response_200 = response.json()
        for response_200_item_data in (_response_200):
            response_200_item = ListJobsResponse200Item.from_dict(response_200_item_data)



            response_200.append(response_200_item)

        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[List['ListJobsResponse200Item']]:
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
    created_by: Union[Unset, None, str] = UNSET,
    parent_job: Union[Unset, None, str] = UNSET,
    script_path_exact: Union[Unset, None, str] = UNSET,
    script_path_start: Union[Unset, None, str] = UNSET,
    schedule_path: Union[Unset, None, str] = UNSET,
    script_hash: Union[Unset, None, str] = UNSET,
    started_before: Union[Unset, None, datetime.datetime] = UNSET,
    started_after: Union[Unset, None, datetime.datetime] = UNSET,
    created_or_started_before: Union[Unset, None, datetime.datetime] = UNSET,
    running: Union[Unset, None, bool] = UNSET,
    scheduled_for_before_now: Union[Unset, None, bool] = UNSET,
    created_or_started_after: Union[Unset, None, datetime.datetime] = UNSET,
    job_kinds: Union[Unset, None, str] = UNSET,
    args: Union[Unset, None, str] = UNSET,
    tag: Union[Unset, None, str] = UNSET,
    result: Union[Unset, None, str] = UNSET,
    is_skipped: Union[Unset, None, bool] = UNSET,
    is_flow_step: Union[Unset, None, bool] = UNSET,
    success: Union[Unset, None, bool] = UNSET,

) -> Response[List['ListJobsResponse200Item']]:
    """ list all jobs

    Args:
        workspace (str):
        created_by (Union[Unset, None, str]):
        parent_job (Union[Unset, None, str]):
        script_path_exact (Union[Unset, None, str]):
        script_path_start (Union[Unset, None, str]):
        schedule_path (Union[Unset, None, str]):
        script_hash (Union[Unset, None, str]):
        started_before (Union[Unset, None, datetime.datetime]):
        started_after (Union[Unset, None, datetime.datetime]):
        created_or_started_before (Union[Unset, None, datetime.datetime]):
        running (Union[Unset, None, bool]):
        scheduled_for_before_now (Union[Unset, None, bool]):
        created_or_started_after (Union[Unset, None, datetime.datetime]):
        job_kinds (Union[Unset, None, str]):
        args (Union[Unset, None, str]):
        tag (Union[Unset, None, str]):
        result (Union[Unset, None, str]):
        is_skipped (Union[Unset, None, bool]):
        is_flow_step (Union[Unset, None, bool]):
        success (Union[Unset, None, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[List['ListJobsResponse200Item']]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
created_by=created_by,
parent_job=parent_job,
script_path_exact=script_path_exact,
script_path_start=script_path_start,
schedule_path=schedule_path,
script_hash=script_hash,
started_before=started_before,
started_after=started_after,
created_or_started_before=created_or_started_before,
running=running,
scheduled_for_before_now=scheduled_for_before_now,
created_or_started_after=created_or_started_after,
job_kinds=job_kinds,
args=args,
tag=tag,
result=result,
is_skipped=is_skipped,
is_flow_step=is_flow_step,
success=success,

    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)

def sync(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    created_by: Union[Unset, None, str] = UNSET,
    parent_job: Union[Unset, None, str] = UNSET,
    script_path_exact: Union[Unset, None, str] = UNSET,
    script_path_start: Union[Unset, None, str] = UNSET,
    schedule_path: Union[Unset, None, str] = UNSET,
    script_hash: Union[Unset, None, str] = UNSET,
    started_before: Union[Unset, None, datetime.datetime] = UNSET,
    started_after: Union[Unset, None, datetime.datetime] = UNSET,
    created_or_started_before: Union[Unset, None, datetime.datetime] = UNSET,
    running: Union[Unset, None, bool] = UNSET,
    scheduled_for_before_now: Union[Unset, None, bool] = UNSET,
    created_or_started_after: Union[Unset, None, datetime.datetime] = UNSET,
    job_kinds: Union[Unset, None, str] = UNSET,
    args: Union[Unset, None, str] = UNSET,
    tag: Union[Unset, None, str] = UNSET,
    result: Union[Unset, None, str] = UNSET,
    is_skipped: Union[Unset, None, bool] = UNSET,
    is_flow_step: Union[Unset, None, bool] = UNSET,
    success: Union[Unset, None, bool] = UNSET,

) -> Optional[List['ListJobsResponse200Item']]:
    """ list all jobs

    Args:
        workspace (str):
        created_by (Union[Unset, None, str]):
        parent_job (Union[Unset, None, str]):
        script_path_exact (Union[Unset, None, str]):
        script_path_start (Union[Unset, None, str]):
        schedule_path (Union[Unset, None, str]):
        script_hash (Union[Unset, None, str]):
        started_before (Union[Unset, None, datetime.datetime]):
        started_after (Union[Unset, None, datetime.datetime]):
        created_or_started_before (Union[Unset, None, datetime.datetime]):
        running (Union[Unset, None, bool]):
        scheduled_for_before_now (Union[Unset, None, bool]):
        created_or_started_after (Union[Unset, None, datetime.datetime]):
        job_kinds (Union[Unset, None, str]):
        args (Union[Unset, None, str]):
        tag (Union[Unset, None, str]):
        result (Union[Unset, None, str]):
        is_skipped (Union[Unset, None, bool]):
        is_flow_step (Union[Unset, None, bool]):
        success (Union[Unset, None, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        List['ListJobsResponse200Item']
     """


    return sync_detailed(
        workspace=workspace,
client=client,
created_by=created_by,
parent_job=parent_job,
script_path_exact=script_path_exact,
script_path_start=script_path_start,
schedule_path=schedule_path,
script_hash=script_hash,
started_before=started_before,
started_after=started_after,
created_or_started_before=created_or_started_before,
running=running,
scheduled_for_before_now=scheduled_for_before_now,
created_or_started_after=created_or_started_after,
job_kinds=job_kinds,
args=args,
tag=tag,
result=result,
is_skipped=is_skipped,
is_flow_step=is_flow_step,
success=success,

    ).parsed

async def asyncio_detailed(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    created_by: Union[Unset, None, str] = UNSET,
    parent_job: Union[Unset, None, str] = UNSET,
    script_path_exact: Union[Unset, None, str] = UNSET,
    script_path_start: Union[Unset, None, str] = UNSET,
    schedule_path: Union[Unset, None, str] = UNSET,
    script_hash: Union[Unset, None, str] = UNSET,
    started_before: Union[Unset, None, datetime.datetime] = UNSET,
    started_after: Union[Unset, None, datetime.datetime] = UNSET,
    created_or_started_before: Union[Unset, None, datetime.datetime] = UNSET,
    running: Union[Unset, None, bool] = UNSET,
    scheduled_for_before_now: Union[Unset, None, bool] = UNSET,
    created_or_started_after: Union[Unset, None, datetime.datetime] = UNSET,
    job_kinds: Union[Unset, None, str] = UNSET,
    args: Union[Unset, None, str] = UNSET,
    tag: Union[Unset, None, str] = UNSET,
    result: Union[Unset, None, str] = UNSET,
    is_skipped: Union[Unset, None, bool] = UNSET,
    is_flow_step: Union[Unset, None, bool] = UNSET,
    success: Union[Unset, None, bool] = UNSET,

) -> Response[List['ListJobsResponse200Item']]:
    """ list all jobs

    Args:
        workspace (str):
        created_by (Union[Unset, None, str]):
        parent_job (Union[Unset, None, str]):
        script_path_exact (Union[Unset, None, str]):
        script_path_start (Union[Unset, None, str]):
        schedule_path (Union[Unset, None, str]):
        script_hash (Union[Unset, None, str]):
        started_before (Union[Unset, None, datetime.datetime]):
        started_after (Union[Unset, None, datetime.datetime]):
        created_or_started_before (Union[Unset, None, datetime.datetime]):
        running (Union[Unset, None, bool]):
        scheduled_for_before_now (Union[Unset, None, bool]):
        created_or_started_after (Union[Unset, None, datetime.datetime]):
        job_kinds (Union[Unset, None, str]):
        args (Union[Unset, None, str]):
        tag (Union[Unset, None, str]):
        result (Union[Unset, None, str]):
        is_skipped (Union[Unset, None, bool]):
        is_flow_step (Union[Unset, None, bool]):
        success (Union[Unset, None, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[List['ListJobsResponse200Item']]
     """


    kwargs = _get_kwargs(
        workspace=workspace,
created_by=created_by,
parent_job=parent_job,
script_path_exact=script_path_exact,
script_path_start=script_path_start,
schedule_path=schedule_path,
script_hash=script_hash,
started_before=started_before,
started_after=started_after,
created_or_started_before=created_or_started_before,
running=running,
scheduled_for_before_now=scheduled_for_before_now,
created_or_started_after=created_or_started_after,
job_kinds=job_kinds,
args=args,
tag=tag,
result=result,
is_skipped=is_skipped,
is_flow_step=is_flow_step,
success=success,

    )

    response = await client.get_async_httpx_client().request(
        **kwargs
    )

    return _build_response(client=client, response=response)

async def asyncio(
    workspace: str,
    *,
    client: Union[AuthenticatedClient, Client],
    created_by: Union[Unset, None, str] = UNSET,
    parent_job: Union[Unset, None, str] = UNSET,
    script_path_exact: Union[Unset, None, str] = UNSET,
    script_path_start: Union[Unset, None, str] = UNSET,
    schedule_path: Union[Unset, None, str] = UNSET,
    script_hash: Union[Unset, None, str] = UNSET,
    started_before: Union[Unset, None, datetime.datetime] = UNSET,
    started_after: Union[Unset, None, datetime.datetime] = UNSET,
    created_or_started_before: Union[Unset, None, datetime.datetime] = UNSET,
    running: Union[Unset, None, bool] = UNSET,
    scheduled_for_before_now: Union[Unset, None, bool] = UNSET,
    created_or_started_after: Union[Unset, None, datetime.datetime] = UNSET,
    job_kinds: Union[Unset, None, str] = UNSET,
    args: Union[Unset, None, str] = UNSET,
    tag: Union[Unset, None, str] = UNSET,
    result: Union[Unset, None, str] = UNSET,
    is_skipped: Union[Unset, None, bool] = UNSET,
    is_flow_step: Union[Unset, None, bool] = UNSET,
    success: Union[Unset, None, bool] = UNSET,

) -> Optional[List['ListJobsResponse200Item']]:
    """ list all jobs

    Args:
        workspace (str):
        created_by (Union[Unset, None, str]):
        parent_job (Union[Unset, None, str]):
        script_path_exact (Union[Unset, None, str]):
        script_path_start (Union[Unset, None, str]):
        schedule_path (Union[Unset, None, str]):
        script_hash (Union[Unset, None, str]):
        started_before (Union[Unset, None, datetime.datetime]):
        started_after (Union[Unset, None, datetime.datetime]):
        created_or_started_before (Union[Unset, None, datetime.datetime]):
        running (Union[Unset, None, bool]):
        scheduled_for_before_now (Union[Unset, None, bool]):
        created_or_started_after (Union[Unset, None, datetime.datetime]):
        job_kinds (Union[Unset, None, str]):
        args (Union[Unset, None, str]):
        tag (Union[Unset, None, str]):
        result (Union[Unset, None, str]):
        is_skipped (Union[Unset, None, bool]):
        is_flow_step (Union[Unset, None, bool]):
        success (Union[Unset, None, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        List['ListJobsResponse200Item']
     """


    return (await asyncio_detailed(
        workspace=workspace,
client=client,
created_by=created_by,
parent_job=parent_job,
script_path_exact=script_path_exact,
script_path_start=script_path_start,
schedule_path=schedule_path,
script_hash=script_hash,
started_before=started_before,
started_after=started_after,
created_or_started_before=created_or_started_before,
running=running,
scheduled_for_before_now=scheduled_for_before_now,
created_or_started_after=created_or_started_after,
job_kinds=job_kinds,
args=args,
tag=tag,
result=result,
is_skipped=is_skipped,
is_flow_step=is_flow_step,
success=success,

    )).parsed
