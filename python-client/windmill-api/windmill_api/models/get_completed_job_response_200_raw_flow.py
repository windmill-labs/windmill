from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.get_completed_job_response_200_raw_flow_failure_module import (
    GetCompletedJobResponse200RawFlowFailureModule,
)
from ..models.get_completed_job_response_200_raw_flow_modules_item import GetCompletedJobResponse200RawFlowModulesItem
from ..types import UNSET, Unset

T = TypeVar("T", bound="GetCompletedJobResponse200RawFlow")


@attr.s(auto_attribs=True)
class GetCompletedJobResponse200RawFlow:
    """
    Attributes:
        modules (List[GetCompletedJobResponse200RawFlowModulesItem]):
        failure_module (Union[Unset, GetCompletedJobResponse200RawFlowFailureModule]):
        same_worker (Union[Unset, bool]):
    """

    modules: List[GetCompletedJobResponse200RawFlowModulesItem]
    failure_module: Union[Unset, GetCompletedJobResponse200RawFlowFailureModule] = UNSET
    same_worker: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        modules = []
        for modules_item_data in self.modules:
            modules_item = modules_item_data.to_dict()

            modules.append(modules_item)

        failure_module: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.failure_module, Unset):
            failure_module = self.failure_module.to_dict()

        same_worker = self.same_worker

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "modules": modules,
            }
        )
        if failure_module is not UNSET:
            field_dict["failure_module"] = failure_module
        if same_worker is not UNSET:
            field_dict["same_worker"] = same_worker

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        modules = []
        _modules = d.pop("modules")
        for modules_item_data in _modules:
            modules_item = GetCompletedJobResponse200RawFlowModulesItem.from_dict(modules_item_data)

            modules.append(modules_item)

        _failure_module = d.pop("failure_module", UNSET)
        failure_module: Union[Unset, GetCompletedJobResponse200RawFlowFailureModule]
        if isinstance(_failure_module, Unset):
            failure_module = UNSET
        else:
            failure_module = GetCompletedJobResponse200RawFlowFailureModule.from_dict(_failure_module)

        same_worker = d.pop("same_worker", UNSET)

        get_completed_job_response_200_raw_flow = cls(
            modules=modules,
            failure_module=failure_module,
            same_worker=same_worker,
        )

        get_completed_job_response_200_raw_flow.additional_properties = d
        return get_completed_job_response_200_raw_flow

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
