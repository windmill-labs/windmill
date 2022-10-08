from typing import Any, Dict, List, Type, TypeVar, Union, cast

import attr

from ..models.list_queue_response_200_item_flow_status_modules_item_iterator import (
    ListQueueResponse200ItemFlowStatusModulesItemIterator,
)
from ..models.list_queue_response_200_item_flow_status_modules_item_type import (
    ListQueueResponse200ItemFlowStatusModulesItemType,
)
from ..types import UNSET, Unset

T = TypeVar("T", bound="ListQueueResponse200ItemFlowStatusModulesItem")


@attr.s(auto_attribs=True)
class ListQueueResponse200ItemFlowStatusModulesItem:
    """
    Attributes:
        type (ListQueueResponse200ItemFlowStatusModulesItemType):
        job (Union[Unset, str]):
        count (Union[Unset, int]):
        iterator (Union[Unset, ListQueueResponse200ItemFlowStatusModulesItemIterator]):
        forloop_jobs (Union[Unset, List[str]]):
    """

    type: ListQueueResponse200ItemFlowStatusModulesItemType
    job: Union[Unset, str] = UNSET
    count: Union[Unset, int] = UNSET
    iterator: Union[Unset, ListQueueResponse200ItemFlowStatusModulesItemIterator] = UNSET
    forloop_jobs: Union[Unset, List[str]] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        type = self.type.value

        job = self.job
        count = self.count
        iterator: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.iterator, Unset):
            iterator = self.iterator.to_dict()

        forloop_jobs: Union[Unset, List[str]] = UNSET
        if not isinstance(self.forloop_jobs, Unset):
            forloop_jobs = self.forloop_jobs

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "type": type,
            }
        )
        if job is not UNSET:
            field_dict["job"] = job
        if count is not UNSET:
            field_dict["count"] = count
        if iterator is not UNSET:
            field_dict["iterator"] = iterator
        if forloop_jobs is not UNSET:
            field_dict["forloop_jobs"] = forloop_jobs

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        type = ListQueueResponse200ItemFlowStatusModulesItemType(d.pop("type"))

        job = d.pop("job", UNSET)

        count = d.pop("count", UNSET)

        _iterator = d.pop("iterator", UNSET)
        iterator: Union[Unset, ListQueueResponse200ItemFlowStatusModulesItemIterator]
        if isinstance(_iterator, Unset):
            iterator = UNSET
        else:
            iterator = ListQueueResponse200ItemFlowStatusModulesItemIterator.from_dict(_iterator)

        forloop_jobs = cast(List[str], d.pop("forloop_jobs", UNSET))

        list_queue_response_200_item_flow_status_modules_item = cls(
            type=type,
            job=job,
            count=count,
            iterator=iterator,
            forloop_jobs=forloop_jobs,
        )

        list_queue_response_200_item_flow_status_modules_item.additional_properties = d
        return list_queue_response_200_item_flow_status_modules_item

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
