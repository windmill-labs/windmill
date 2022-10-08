import datetime
from typing import Any, Dict, List, Type, TypeVar, Union

import attr
from dateutil.parser import isoparse

from ..models.schedule_args import ScheduleArgs
from ..models.schedule_extra_perms import ScheduleExtraPerms
from ..types import UNSET, Unset

T = TypeVar("T", bound="Schedule")


@attr.s(auto_attribs=True)
class Schedule:
    """
    Attributes:
        path (str):
        edited_by (str):
        edited_at (datetime.datetime):
        schedule (str):
        offset (int):
        enabled (bool):
        script_path (str):
        is_flow (bool):
        extra_perms (ScheduleExtraPerms):
        args (Union[Unset, ScheduleArgs]):
    """

    path: str
    edited_by: str
    edited_at: datetime.datetime
    schedule: str
    offset: int
    enabled: bool
    script_path: str
    is_flow: bool
    extra_perms: ScheduleExtraPerms
    args: Union[Unset, ScheduleArgs] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        path = self.path
        edited_by = self.edited_by
        edited_at = self.edited_at.isoformat()

        schedule = self.schedule
        offset = self.offset
        enabled = self.enabled
        script_path = self.script_path
        is_flow = self.is_flow
        extra_perms = self.extra_perms.to_dict()

        args: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.args, Unset):
            args = self.args.to_dict()

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "path": path,
                "edited_by": edited_by,
                "edited_at": edited_at,
                "schedule": schedule,
                "offset_": offset,
                "enabled": enabled,
                "script_path": script_path,
                "is_flow": is_flow,
                "extra_perms": extra_perms,
            }
        )
        if args is not UNSET:
            field_dict["args"] = args

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        path = d.pop("path")

        edited_by = d.pop("edited_by")

        edited_at = isoparse(d.pop("edited_at"))

        schedule = d.pop("schedule")

        offset = d.pop("offset_")

        enabled = d.pop("enabled")

        script_path = d.pop("script_path")

        is_flow = d.pop("is_flow")

        extra_perms = ScheduleExtraPerms.from_dict(d.pop("extra_perms"))

        _args = d.pop("args", UNSET)
        args: Union[Unset, ScheduleArgs]
        if isinstance(_args, Unset):
            args = UNSET
        else:
            args = ScheduleArgs.from_dict(_args)

        schedule = cls(
            path=path,
            edited_by=edited_by,
            edited_at=edited_at,
            schedule=schedule,
            offset=offset,
            enabled=enabled,
            script_path=script_path,
            is_flow=is_flow,
            extra_perms=extra_perms,
            args=args,
        )

        schedule.additional_properties = d
        return schedule

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
