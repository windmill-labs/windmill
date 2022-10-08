import datetime
from typing import Any, Dict, List, Type, TypeVar, Union

import attr
from dateutil.parser import isoparse

from ..models.list_completed_jobs_response_200_item_args import ListCompletedJobsResponse200ItemArgs
from ..models.list_completed_jobs_response_200_item_flow_status import ListCompletedJobsResponse200ItemFlowStatus
from ..models.list_completed_jobs_response_200_item_job_kind import ListCompletedJobsResponse200ItemJobKind
from ..models.list_completed_jobs_response_200_item_language import ListCompletedJobsResponse200ItemLanguage
from ..models.list_completed_jobs_response_200_item_raw_flow import ListCompletedJobsResponse200ItemRawFlow
from ..models.list_completed_jobs_response_200_item_result import ListCompletedJobsResponse200ItemResult
from ..types import UNSET, Unset

T = TypeVar("T", bound="ListCompletedJobsResponse200Item")


@attr.s(auto_attribs=True)
class ListCompletedJobsResponse200Item:
    """
    Attributes:
        id (str):
        created_by (str):
        created_at (datetime.datetime):
        started_at (datetime.datetime):
        duration_ms (int):
        success (bool):
        canceled (bool):
        job_kind (ListCompletedJobsResponse200ItemJobKind):
        permissioned_as (str): The user (u/userfoo) or group (g/groupfoo) whom
            the execution of this script will be permissioned_as and by extension its DT_TOKEN.
        is_flow_step (bool):
        is_skipped (bool):
        workspace_id (Union[Unset, str]):
        parent_job (Union[Unset, str]):
        script_path (Union[Unset, str]):
        script_hash (Union[Unset, str]):
        args (Union[Unset, ListCompletedJobsResponse200ItemArgs]):
        result (Union[Unset, ListCompletedJobsResponse200ItemResult]):
        logs (Union[Unset, str]):
        deleted (Union[Unset, bool]):
        raw_code (Union[Unset, str]):
        canceled_by (Union[Unset, str]):
        canceled_reason (Union[Unset, str]):
        schedule_path (Union[Unset, str]):
        flow_status (Union[Unset, ListCompletedJobsResponse200ItemFlowStatus]):
        raw_flow (Union[Unset, ListCompletedJobsResponse200ItemRawFlow]):
        language (Union[Unset, ListCompletedJobsResponse200ItemLanguage]):
    """

    id: str
    created_by: str
    created_at: datetime.datetime
    started_at: datetime.datetime
    duration_ms: int
    success: bool
    canceled: bool
    job_kind: ListCompletedJobsResponse200ItemJobKind
    permissioned_as: str
    is_flow_step: bool
    is_skipped: bool
    workspace_id: Union[Unset, str] = UNSET
    parent_job: Union[Unset, str] = UNSET
    script_path: Union[Unset, str] = UNSET
    script_hash: Union[Unset, str] = UNSET
    args: Union[Unset, ListCompletedJobsResponse200ItemArgs] = UNSET
    result: Union[Unset, ListCompletedJobsResponse200ItemResult] = UNSET
    logs: Union[Unset, str] = UNSET
    deleted: Union[Unset, bool] = UNSET
    raw_code: Union[Unset, str] = UNSET
    canceled_by: Union[Unset, str] = UNSET
    canceled_reason: Union[Unset, str] = UNSET
    schedule_path: Union[Unset, str] = UNSET
    flow_status: Union[Unset, ListCompletedJobsResponse200ItemFlowStatus] = UNSET
    raw_flow: Union[Unset, ListCompletedJobsResponse200ItemRawFlow] = UNSET
    language: Union[Unset, ListCompletedJobsResponse200ItemLanguage] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        id = self.id
        created_by = self.created_by
        created_at = self.created_at.isoformat()

        started_at = self.started_at.isoformat()

        duration_ms = self.duration_ms
        success = self.success
        canceled = self.canceled
        job_kind = self.job_kind.value

        permissioned_as = self.permissioned_as
        is_flow_step = self.is_flow_step
        is_skipped = self.is_skipped
        workspace_id = self.workspace_id
        parent_job = self.parent_job
        script_path = self.script_path
        script_hash = self.script_hash
        args: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.args, Unset):
            args = self.args.to_dict()

        result: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.result, Unset):
            result = self.result.to_dict()

        logs = self.logs
        deleted = self.deleted
        raw_code = self.raw_code
        canceled_by = self.canceled_by
        canceled_reason = self.canceled_reason
        schedule_path = self.schedule_path
        flow_status: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.flow_status, Unset):
            flow_status = self.flow_status.to_dict()

        raw_flow: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.raw_flow, Unset):
            raw_flow = self.raw_flow.to_dict()

        language: Union[Unset, str] = UNSET
        if not isinstance(self.language, Unset):
            language = self.language.value

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "created_by": created_by,
                "created_at": created_at,
                "started_at": started_at,
                "duration_ms": duration_ms,
                "success": success,
                "canceled": canceled,
                "job_kind": job_kind,
                "permissioned_as": permissioned_as,
                "is_flow_step": is_flow_step,
                "is_skipped": is_skipped,
            }
        )
        if workspace_id is not UNSET:
            field_dict["workspace_id"] = workspace_id
        if parent_job is not UNSET:
            field_dict["parent_job"] = parent_job
        if script_path is not UNSET:
            field_dict["script_path"] = script_path
        if script_hash is not UNSET:
            field_dict["script_hash"] = script_hash
        if args is not UNSET:
            field_dict["args"] = args
        if result is not UNSET:
            field_dict["result"] = result
        if logs is not UNSET:
            field_dict["logs"] = logs
        if deleted is not UNSET:
            field_dict["deleted"] = deleted
        if raw_code is not UNSET:
            field_dict["raw_code"] = raw_code
        if canceled_by is not UNSET:
            field_dict["canceled_by"] = canceled_by
        if canceled_reason is not UNSET:
            field_dict["canceled_reason"] = canceled_reason
        if schedule_path is not UNSET:
            field_dict["schedule_path"] = schedule_path
        if flow_status is not UNSET:
            field_dict["flow_status"] = flow_status
        if raw_flow is not UNSET:
            field_dict["raw_flow"] = raw_flow
        if language is not UNSET:
            field_dict["language"] = language

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        id = d.pop("id")

        created_by = d.pop("created_by")

        created_at = isoparse(d.pop("created_at"))

        started_at = isoparse(d.pop("started_at"))

        duration_ms = d.pop("duration_ms")

        success = d.pop("success")

        canceled = d.pop("canceled")

        job_kind = ListCompletedJobsResponse200ItemJobKind(d.pop("job_kind"))

        permissioned_as = d.pop("permissioned_as")

        is_flow_step = d.pop("is_flow_step")

        is_skipped = d.pop("is_skipped")

        workspace_id = d.pop("workspace_id", UNSET)

        parent_job = d.pop("parent_job", UNSET)

        script_path = d.pop("script_path", UNSET)

        script_hash = d.pop("script_hash", UNSET)

        _args = d.pop("args", UNSET)
        args: Union[Unset, ListCompletedJobsResponse200ItemArgs]
        if isinstance(_args, Unset):
            args = UNSET
        else:
            args = ListCompletedJobsResponse200ItemArgs.from_dict(_args)

        _result = d.pop("result", UNSET)
        result: Union[Unset, ListCompletedJobsResponse200ItemResult]
        if isinstance(_result, Unset):
            result = UNSET
        else:
            result = ListCompletedJobsResponse200ItemResult.from_dict(_result)

        logs = d.pop("logs", UNSET)

        deleted = d.pop("deleted", UNSET)

        raw_code = d.pop("raw_code", UNSET)

        canceled_by = d.pop("canceled_by", UNSET)

        canceled_reason = d.pop("canceled_reason", UNSET)

        schedule_path = d.pop("schedule_path", UNSET)

        _flow_status = d.pop("flow_status", UNSET)
        flow_status: Union[Unset, ListCompletedJobsResponse200ItemFlowStatus]
        if isinstance(_flow_status, Unset):
            flow_status = UNSET
        else:
            flow_status = ListCompletedJobsResponse200ItemFlowStatus.from_dict(_flow_status)

        _raw_flow = d.pop("raw_flow", UNSET)
        raw_flow: Union[Unset, ListCompletedJobsResponse200ItemRawFlow]
        if isinstance(_raw_flow, Unset):
            raw_flow = UNSET
        else:
            raw_flow = ListCompletedJobsResponse200ItemRawFlow.from_dict(_raw_flow)

        _language = d.pop("language", UNSET)
        language: Union[Unset, ListCompletedJobsResponse200ItemLanguage]
        if isinstance(_language, Unset):
            language = UNSET
        else:
            language = ListCompletedJobsResponse200ItemLanguage(_language)

        list_completed_jobs_response_200_item = cls(
            id=id,
            created_by=created_by,
            created_at=created_at,
            started_at=started_at,
            duration_ms=duration_ms,
            success=success,
            canceled=canceled,
            job_kind=job_kind,
            permissioned_as=permissioned_as,
            is_flow_step=is_flow_step,
            is_skipped=is_skipped,
            workspace_id=workspace_id,
            parent_job=parent_job,
            script_path=script_path,
            script_hash=script_hash,
            args=args,
            result=result,
            logs=logs,
            deleted=deleted,
            raw_code=raw_code,
            canceled_by=canceled_by,
            canceled_reason=canceled_reason,
            schedule_path=schedule_path,
            flow_status=flow_status,
            raw_flow=raw_flow,
            language=language,
        )

        list_completed_jobs_response_200_item.additional_properties = d
        return list_completed_jobs_response_200_item

    @property
    def additional_keys(self) -> List[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
