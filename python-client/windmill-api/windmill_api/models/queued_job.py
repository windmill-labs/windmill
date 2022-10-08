import datetime
from typing import Any, Dict, List, Type, TypeVar, Union

import attr
from dateutil.parser import isoparse

from ..models.queued_job_args import QueuedJobArgs
from ..models.queued_job_flow_status import QueuedJobFlowStatus
from ..models.queued_job_job_kind import QueuedJobJobKind
from ..models.queued_job_language import QueuedJobLanguage
from ..models.queued_job_raw_flow import QueuedJobRawFlow
from ..types import UNSET, Unset

T = TypeVar("T", bound="QueuedJob")


@attr.s(auto_attribs=True)
class QueuedJob:
    """
    Attributes:
        id (str):
        running (bool):
        canceled (bool):
        job_kind (QueuedJobJobKind):
        permissioned_as (str): The user (u/userfoo) or group (g/groupfoo) whom
            the execution of this script will be permissioned_as and by extension its DT_TOKEN.
        is_flow_step (bool):
        workspace_id (Union[Unset, str]):
        parent_job (Union[Unset, str]):
        created_by (Union[Unset, str]):
        created_at (Union[Unset, datetime.datetime]):
        started_at (Union[Unset, datetime.datetime]):
        scheduled_for (Union[Unset, datetime.datetime]):
        script_path (Union[Unset, str]):
        script_hash (Union[Unset, str]):
        args (Union[Unset, QueuedJobArgs]):
        logs (Union[Unset, str]):
        raw_code (Union[Unset, str]):
        canceled_by (Union[Unset, str]):
        canceled_reason (Union[Unset, str]):
        last_ping (Union[Unset, datetime.datetime]):
        schedule_path (Union[Unset, str]):
        flow_status (Union[Unset, QueuedJobFlowStatus]):
        raw_flow (Union[Unset, QueuedJobRawFlow]):
        language (Union[Unset, QueuedJobLanguage]):
    """

    id: str
    running: bool
    canceled: bool
    job_kind: QueuedJobJobKind
    permissioned_as: str
    is_flow_step: bool
    workspace_id: Union[Unset, str] = UNSET
    parent_job: Union[Unset, str] = UNSET
    created_by: Union[Unset, str] = UNSET
    created_at: Union[Unset, datetime.datetime] = UNSET
    started_at: Union[Unset, datetime.datetime] = UNSET
    scheduled_for: Union[Unset, datetime.datetime] = UNSET
    script_path: Union[Unset, str] = UNSET
    script_hash: Union[Unset, str] = UNSET
    args: Union[Unset, QueuedJobArgs] = UNSET
    logs: Union[Unset, str] = UNSET
    raw_code: Union[Unset, str] = UNSET
    canceled_by: Union[Unset, str] = UNSET
    canceled_reason: Union[Unset, str] = UNSET
    last_ping: Union[Unset, datetime.datetime] = UNSET
    schedule_path: Union[Unset, str] = UNSET
    flow_status: Union[Unset, QueuedJobFlowStatus] = UNSET
    raw_flow: Union[Unset, QueuedJobRawFlow] = UNSET
    language: Union[Unset, QueuedJobLanguage] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        id = self.id
        running = self.running
        canceled = self.canceled
        job_kind = self.job_kind.value

        permissioned_as = self.permissioned_as
        is_flow_step = self.is_flow_step
        workspace_id = self.workspace_id
        parent_job = self.parent_job
        created_by = self.created_by
        created_at: Union[Unset, str] = UNSET
        if not isinstance(self.created_at, Unset):
            created_at = self.created_at.isoformat()

        started_at: Union[Unset, str] = UNSET
        if not isinstance(self.started_at, Unset):
            started_at = self.started_at.isoformat()

        scheduled_for: Union[Unset, str] = UNSET
        if not isinstance(self.scheduled_for, Unset):
            scheduled_for = self.scheduled_for.isoformat()

        script_path = self.script_path
        script_hash = self.script_hash
        args: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.args, Unset):
            args = self.args.to_dict()

        logs = self.logs
        raw_code = self.raw_code
        canceled_by = self.canceled_by
        canceled_reason = self.canceled_reason
        last_ping: Union[Unset, str] = UNSET
        if not isinstance(self.last_ping, Unset):
            last_ping = self.last_ping.isoformat()

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
                "running": running,
                "canceled": canceled,
                "job_kind": job_kind,
                "permissioned_as": permissioned_as,
                "is_flow_step": is_flow_step,
            }
        )
        if workspace_id is not UNSET:
            field_dict["workspace_id"] = workspace_id
        if parent_job is not UNSET:
            field_dict["parent_job"] = parent_job
        if created_by is not UNSET:
            field_dict["created_by"] = created_by
        if created_at is not UNSET:
            field_dict["created_at"] = created_at
        if started_at is not UNSET:
            field_dict["started_at"] = started_at
        if scheduled_for is not UNSET:
            field_dict["scheduled_for"] = scheduled_for
        if script_path is not UNSET:
            field_dict["script_path"] = script_path
        if script_hash is not UNSET:
            field_dict["script_hash"] = script_hash
        if args is not UNSET:
            field_dict["args"] = args
        if logs is not UNSET:
            field_dict["logs"] = logs
        if raw_code is not UNSET:
            field_dict["raw_code"] = raw_code
        if canceled_by is not UNSET:
            field_dict["canceled_by"] = canceled_by
        if canceled_reason is not UNSET:
            field_dict["canceled_reason"] = canceled_reason
        if last_ping is not UNSET:
            field_dict["last_ping"] = last_ping
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

        running = d.pop("running")

        canceled = d.pop("canceled")

        job_kind = QueuedJobJobKind(d.pop("job_kind"))

        permissioned_as = d.pop("permissioned_as")

        is_flow_step = d.pop("is_flow_step")

        workspace_id = d.pop("workspace_id", UNSET)

        parent_job = d.pop("parent_job", UNSET)

        created_by = d.pop("created_by", UNSET)

        _created_at = d.pop("created_at", UNSET)
        created_at: Union[Unset, datetime.datetime]
        if isinstance(_created_at, Unset):
            created_at = UNSET
        else:
            created_at = isoparse(_created_at)

        _started_at = d.pop("started_at", UNSET)
        started_at: Union[Unset, datetime.datetime]
        if isinstance(_started_at, Unset):
            started_at = UNSET
        else:
            started_at = isoparse(_started_at)

        _scheduled_for = d.pop("scheduled_for", UNSET)
        scheduled_for: Union[Unset, datetime.datetime]
        if isinstance(_scheduled_for, Unset):
            scheduled_for = UNSET
        else:
            scheduled_for = isoparse(_scheduled_for)

        script_path = d.pop("script_path", UNSET)

        script_hash = d.pop("script_hash", UNSET)

        _args = d.pop("args", UNSET)
        args: Union[Unset, QueuedJobArgs]
        if isinstance(_args, Unset):
            args = UNSET
        else:
            args = QueuedJobArgs.from_dict(_args)

        logs = d.pop("logs", UNSET)

        raw_code = d.pop("raw_code", UNSET)

        canceled_by = d.pop("canceled_by", UNSET)

        canceled_reason = d.pop("canceled_reason", UNSET)

        _last_ping = d.pop("last_ping", UNSET)
        last_ping: Union[Unset, datetime.datetime]
        if isinstance(_last_ping, Unset):
            last_ping = UNSET
        else:
            last_ping = isoparse(_last_ping)

        schedule_path = d.pop("schedule_path", UNSET)

        _flow_status = d.pop("flow_status", UNSET)
        flow_status: Union[Unset, QueuedJobFlowStatus]
        if isinstance(_flow_status, Unset):
            flow_status = UNSET
        else:
            flow_status = QueuedJobFlowStatus.from_dict(_flow_status)

        _raw_flow = d.pop("raw_flow", UNSET)
        raw_flow: Union[Unset, QueuedJobRawFlow]
        if isinstance(_raw_flow, Unset):
            raw_flow = UNSET
        else:
            raw_flow = QueuedJobRawFlow.from_dict(_raw_flow)

        _language = d.pop("language", UNSET)
        language: Union[Unset, QueuedJobLanguage]
        if isinstance(_language, Unset):
            language = UNSET
        else:
            language = QueuedJobLanguage(_language)

        queued_job = cls(
            id=id,
            running=running,
            canceled=canceled,
            job_kind=job_kind,
            permissioned_as=permissioned_as,
            is_flow_step=is_flow_step,
            workspace_id=workspace_id,
            parent_job=parent_job,
            created_by=created_by,
            created_at=created_at,
            started_at=started_at,
            scheduled_for=scheduled_for,
            script_path=script_path,
            script_hash=script_hash,
            args=args,
            logs=logs,
            raw_code=raw_code,
            canceled_by=canceled_by,
            canceled_reason=canceled_reason,
            last_ping=last_ping,
            schedule_path=schedule_path,
            flow_status=flow_status,
            raw_flow=raw_flow,
            language=language,
        )

        queued_job.additional_properties = d
        return queued_job

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
