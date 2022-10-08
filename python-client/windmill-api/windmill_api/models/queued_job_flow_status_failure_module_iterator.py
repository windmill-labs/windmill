from typing import Any, Dict, List, Type, TypeVar, Union, cast

import attr

from ..types import UNSET, Unset

T = TypeVar("T", bound="QueuedJobFlowStatusFailureModuleIterator")


@attr.s(auto_attribs=True)
class QueuedJobFlowStatusFailureModuleIterator:
    """
    Attributes:
        index (Union[Unset, int]):
        itered (Union[Unset, List[Any]]):
        args (Union[Unset, Any]):
    """

    index: Union[Unset, int] = UNSET
    itered: Union[Unset, List[Any]] = UNSET
    args: Union[Unset, Any] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        index = self.index
        itered: Union[Unset, List[Any]] = UNSET
        if not isinstance(self.itered, Unset):
            itered = self.itered

        args = self.args

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if index is not UNSET:
            field_dict["index"] = index
        if itered is not UNSET:
            field_dict["itered"] = itered
        if args is not UNSET:
            field_dict["args"] = args

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        index = d.pop("index", UNSET)

        itered = cast(List[Any], d.pop("itered", UNSET))

        args = d.pop("args", UNSET)

        queued_job_flow_status_failure_module_iterator = cls(
            index=index,
            itered=itered,
            args=args,
        )

        queued_job_flow_status_failure_module_iterator.additional_properties = d
        return queued_job_flow_status_failure_module_iterator

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
