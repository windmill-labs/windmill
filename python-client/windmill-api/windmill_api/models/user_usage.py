from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..types import UNSET, Unset

T = TypeVar("T", bound="UserUsage")


@attr.s(auto_attribs=True)
class UserUsage:
    """
    Attributes:
        duration_ms (Union[Unset, int]):
        jobs (Union[Unset, int]):
        flows (Union[Unset, int]):
    """

    duration_ms: Union[Unset, int] = UNSET
    jobs: Union[Unset, int] = UNSET
    flows: Union[Unset, int] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        duration_ms = self.duration_ms
        jobs = self.jobs
        flows = self.flows

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if duration_ms is not UNSET:
            field_dict["duration_ms"] = duration_ms
        if jobs is not UNSET:
            field_dict["jobs"] = jobs
        if flows is not UNSET:
            field_dict["flows"] = flows

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        duration_ms = d.pop("duration_ms", UNSET)

        jobs = d.pop("jobs", UNSET)

        flows = d.pop("flows", UNSET)

        user_usage = cls(
            duration_ms=duration_ms,
            jobs=jobs,
            flows=flows,
        )

        user_usage.additional_properties = d
        return user_usage

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
