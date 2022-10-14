from typing import Any, Dict, List, Type, TypeVar, Union, cast

import attr

from ..types import UNSET, Unset

T = TypeVar("T", bound="FlowStatusRetry")


@attr.s(auto_attribs=True)
class FlowStatusRetry:
    """
    Attributes:
        fail_count (Union[Unset, int]):
        failed_jobs (Union[Unset, List[str]]):
    """

    fail_count: Union[Unset, int] = UNSET
    failed_jobs: Union[Unset, List[str]] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        fail_count = self.fail_count
        failed_jobs: Union[Unset, List[str]] = UNSET
        if not isinstance(self.failed_jobs, Unset):
            failed_jobs = self.failed_jobs

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if fail_count is not UNSET:
            field_dict["fail_count"] = fail_count
        if failed_jobs is not UNSET:
            field_dict["failed_jobs"] = failed_jobs

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        fail_count = d.pop("fail_count", UNSET)

        failed_jobs = cast(List[str], d.pop("failed_jobs", UNSET))

        flow_status_retry = cls(
            fail_count=fail_count,
            failed_jobs=failed_jobs,
        )

        flow_status_retry.additional_properties = d
        return flow_status_retry

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
