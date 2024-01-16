from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from ..types import UNSET, Unset
from typing import Dict

if TYPE_CHECKING:
  from ..models.create_schedule_json_body_args import CreateScheduleJsonBodyArgs
  from ..models.create_schedule_json_body_on_failure_extra_args import CreateScheduleJsonBodyOnFailureExtraArgs
  from ..models.create_schedule_json_body_on_recovery_extra_args import CreateScheduleJsonBodyOnRecoveryExtraArgs
  from ..models.create_schedule_json_body_retry import CreateScheduleJsonBodyRetry





T = TypeVar("T", bound="CreateScheduleJsonBody")


@_attrs_define
class CreateScheduleJsonBody:
    """ 
        Attributes:
            path (str):
            schedule (str):
            timezone (str):
            script_path (str):
            is_flow (bool):
            args (CreateScheduleJsonBodyArgs):
            enabled (Union[Unset, bool]):
            on_failure (Union[Unset, str]):
            on_failure_times (Union[Unset, float]):
            on_failure_exact (Union[Unset, bool]):
            on_failure_extra_args (Union[Unset, CreateScheduleJsonBodyOnFailureExtraArgs]):
            on_recovery (Union[Unset, str]):
            on_recovery_times (Union[Unset, float]):
            on_recovery_extra_args (Union[Unset, CreateScheduleJsonBodyOnRecoveryExtraArgs]):
            ws_error_handler_muted (Union[Unset, bool]):
            retry (Union[Unset, CreateScheduleJsonBodyRetry]):
            no_flow_overlap (Union[Unset, bool]):
            summary (Union[Unset, str]):
            tag (Union[Unset, str]):
     """

    path: str
    schedule: str
    timezone: str
    script_path: str
    is_flow: bool
    args: 'CreateScheduleJsonBodyArgs'
    enabled: Union[Unset, bool] = UNSET
    on_failure: Union[Unset, str] = UNSET
    on_failure_times: Union[Unset, float] = UNSET
    on_failure_exact: Union[Unset, bool] = UNSET
    on_failure_extra_args: Union[Unset, 'CreateScheduleJsonBodyOnFailureExtraArgs'] = UNSET
    on_recovery: Union[Unset, str] = UNSET
    on_recovery_times: Union[Unset, float] = UNSET
    on_recovery_extra_args: Union[Unset, 'CreateScheduleJsonBodyOnRecoveryExtraArgs'] = UNSET
    ws_error_handler_muted: Union[Unset, bool] = UNSET
    retry: Union[Unset, 'CreateScheduleJsonBodyRetry'] = UNSET
    no_flow_overlap: Union[Unset, bool] = UNSET
    summary: Union[Unset, str] = UNSET
    tag: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.create_schedule_json_body_args import CreateScheduleJsonBodyArgs
        from ..models.create_schedule_json_body_on_failure_extra_args import CreateScheduleJsonBodyOnFailureExtraArgs
        from ..models.create_schedule_json_body_on_recovery_extra_args import CreateScheduleJsonBodyOnRecoveryExtraArgs
        from ..models.create_schedule_json_body_retry import CreateScheduleJsonBodyRetry
        path = self.path
        schedule = self.schedule
        timezone = self.timezone
        script_path = self.script_path
        is_flow = self.is_flow
        args = self.args.to_dict()

        enabled = self.enabled
        on_failure = self.on_failure
        on_failure_times = self.on_failure_times
        on_failure_exact = self.on_failure_exact
        on_failure_extra_args: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.on_failure_extra_args, Unset):
            on_failure_extra_args = self.on_failure_extra_args.to_dict()

        on_recovery = self.on_recovery
        on_recovery_times = self.on_recovery_times
        on_recovery_extra_args: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.on_recovery_extra_args, Unset):
            on_recovery_extra_args = self.on_recovery_extra_args.to_dict()

        ws_error_handler_muted = self.ws_error_handler_muted
        retry: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.retry, Unset):
            retry = self.retry.to_dict()

        no_flow_overlap = self.no_flow_overlap
        summary = self.summary
        tag = self.tag

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "path": path,
            "schedule": schedule,
            "timezone": timezone,
            "script_path": script_path,
            "is_flow": is_flow,
            "args": args,
        })
        if enabled is not UNSET:
            field_dict["enabled"] = enabled
        if on_failure is not UNSET:
            field_dict["on_failure"] = on_failure
        if on_failure_times is not UNSET:
            field_dict["on_failure_times"] = on_failure_times
        if on_failure_exact is not UNSET:
            field_dict["on_failure_exact"] = on_failure_exact
        if on_failure_extra_args is not UNSET:
            field_dict["on_failure_extra_args"] = on_failure_extra_args
        if on_recovery is not UNSET:
            field_dict["on_recovery"] = on_recovery
        if on_recovery_times is not UNSET:
            field_dict["on_recovery_times"] = on_recovery_times
        if on_recovery_extra_args is not UNSET:
            field_dict["on_recovery_extra_args"] = on_recovery_extra_args
        if ws_error_handler_muted is not UNSET:
            field_dict["ws_error_handler_muted"] = ws_error_handler_muted
        if retry is not UNSET:
            field_dict["retry"] = retry
        if no_flow_overlap is not UNSET:
            field_dict["no_flow_overlap"] = no_flow_overlap
        if summary is not UNSET:
            field_dict["summary"] = summary
        if tag is not UNSET:
            field_dict["tag"] = tag

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.create_schedule_json_body_args import CreateScheduleJsonBodyArgs
        from ..models.create_schedule_json_body_on_failure_extra_args import CreateScheduleJsonBodyOnFailureExtraArgs
        from ..models.create_schedule_json_body_on_recovery_extra_args import CreateScheduleJsonBodyOnRecoveryExtraArgs
        from ..models.create_schedule_json_body_retry import CreateScheduleJsonBodyRetry
        d = src_dict.copy()
        path = d.pop("path")

        schedule = d.pop("schedule")

        timezone = d.pop("timezone")

        script_path = d.pop("script_path")

        is_flow = d.pop("is_flow")

        args = CreateScheduleJsonBodyArgs.from_dict(d.pop("args"))




        enabled = d.pop("enabled", UNSET)

        on_failure = d.pop("on_failure", UNSET)

        on_failure_times = d.pop("on_failure_times", UNSET)

        on_failure_exact = d.pop("on_failure_exact", UNSET)

        _on_failure_extra_args = d.pop("on_failure_extra_args", UNSET)
        on_failure_extra_args: Union[Unset, CreateScheduleJsonBodyOnFailureExtraArgs]
        if isinstance(_on_failure_extra_args,  Unset):
            on_failure_extra_args = UNSET
        else:
            on_failure_extra_args = CreateScheduleJsonBodyOnFailureExtraArgs.from_dict(_on_failure_extra_args)




        on_recovery = d.pop("on_recovery", UNSET)

        on_recovery_times = d.pop("on_recovery_times", UNSET)

        _on_recovery_extra_args = d.pop("on_recovery_extra_args", UNSET)
        on_recovery_extra_args: Union[Unset, CreateScheduleJsonBodyOnRecoveryExtraArgs]
        if isinstance(_on_recovery_extra_args,  Unset):
            on_recovery_extra_args = UNSET
        else:
            on_recovery_extra_args = CreateScheduleJsonBodyOnRecoveryExtraArgs.from_dict(_on_recovery_extra_args)




        ws_error_handler_muted = d.pop("ws_error_handler_muted", UNSET)

        _retry = d.pop("retry", UNSET)
        retry: Union[Unset, CreateScheduleJsonBodyRetry]
        if isinstance(_retry,  Unset):
            retry = UNSET
        else:
            retry = CreateScheduleJsonBodyRetry.from_dict(_retry)




        no_flow_overlap = d.pop("no_flow_overlap", UNSET)

        summary = d.pop("summary", UNSET)

        tag = d.pop("tag", UNSET)

        create_schedule_json_body = cls(
            path=path,
            schedule=schedule,
            timezone=timezone,
            script_path=script_path,
            is_flow=is_flow,
            args=args,
            enabled=enabled,
            on_failure=on_failure,
            on_failure_times=on_failure_times,
            on_failure_exact=on_failure_exact,
            on_failure_extra_args=on_failure_extra_args,
            on_recovery=on_recovery,
            on_recovery_times=on_recovery_times,
            on_recovery_extra_args=on_recovery_extra_args,
            ws_error_handler_muted=ws_error_handler_muted,
            retry=retry,
            no_flow_overlap=no_flow_overlap,
            summary=summary,
            tag=tag,
        )

        create_schedule_json_body.additional_properties = d
        return create_schedule_json_body

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
