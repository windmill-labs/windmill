from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..types import UNSET, Unset

T = TypeVar("T", bound="QueuedJobFlowStatusRetry")


@attr.s(auto_attribs=True)
class QueuedJobFlowStatusRetry:
    """
    Attributes:
        fail_count (Union[Unset, int]):
    """

    fail_count: Union[Unset, int] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        fail_count = self.fail_count

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if fail_count is not UNSET:
            field_dict["fail_count"] = fail_count

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        fail_count = d.pop("fail_count", UNSET)

        queued_job_flow_status_retry = cls(
            fail_count=fail_count,
        )

        queued_job_flow_status_retry.additional_properties = d
        return queued_job_flow_status_retry

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
