from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.create_schedule_json_body_args import CreateScheduleJsonBodyArgs
from ..types import UNSET, Unset

T = TypeVar("T", bound="CreateScheduleJsonBody")


@attr.s(auto_attribs=True)
class CreateScheduleJsonBody:
    """
    Attributes:
        path (str):
        schedule (str):
        script_path (str):
        is_flow (bool):
        args (CreateScheduleJsonBodyArgs):
        offset (Union[Unset, int]):
        enabled (Union[Unset, bool]):
    """

    path: str
    schedule: str
    script_path: str
    is_flow: bool
    args: CreateScheduleJsonBodyArgs
    offset: Union[Unset, int] = UNSET
    enabled: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        path = self.path
        schedule = self.schedule
        script_path = self.script_path
        is_flow = self.is_flow
        args = self.args.to_dict()

        offset = self.offset
        enabled = self.enabled

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "path": path,
                "schedule": schedule,
                "script_path": script_path,
                "is_flow": is_flow,
                "args": args,
            }
        )
        if offset is not UNSET:
            field_dict["offset"] = offset
        if enabled is not UNSET:
            field_dict["enabled"] = enabled

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        path = d.pop("path")

        schedule = d.pop("schedule")

        script_path = d.pop("script_path")

        is_flow = d.pop("is_flow")

        args = CreateScheduleJsonBodyArgs.from_dict(d.pop("args"))

        offset = d.pop("offset", UNSET)

        enabled = d.pop("enabled", UNSET)

        create_schedule_json_body = cls(
            path=path,
            schedule=schedule,
            script_path=script_path,
            is_flow=is_flow,
            args=args,
            offset=offset,
            enabled=enabled,
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
