from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from typing import cast, List
from typing import Dict
from ..types import UNSET, Unset

if TYPE_CHECKING:
  from ..models.completed_job_flow_status_retry import CompletedJobFlowStatusRetry
  from ..models.completed_job_flow_status_failure_module import CompletedJobFlowStatusFailureModule
  from ..models.completed_job_flow_status_modules_item import CompletedJobFlowStatusModulesItem





T = TypeVar("T", bound="CompletedJobFlowStatus")


@_attrs_define
class CompletedJobFlowStatus:
    """ 
        Attributes:
            step (int):
            modules (List['CompletedJobFlowStatusModulesItem']):
            failure_module (CompletedJobFlowStatusFailureModule):
            retry (Union[Unset, CompletedJobFlowStatusRetry]):
     """

    step: int
    modules: List['CompletedJobFlowStatusModulesItem']
    failure_module: 'CompletedJobFlowStatusFailureModule'
    retry: Union[Unset, 'CompletedJobFlowStatusRetry'] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.completed_job_flow_status_retry import CompletedJobFlowStatusRetry
        from ..models.completed_job_flow_status_failure_module import CompletedJobFlowStatusFailureModule
        from ..models.completed_job_flow_status_modules_item import CompletedJobFlowStatusModulesItem
        step = self.step
        modules = []
        for modules_item_data in self.modules:
            modules_item = modules_item_data.to_dict()

            modules.append(modules_item)




        failure_module = self.failure_module.to_dict()

        retry: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.retry, Unset):
            retry = self.retry.to_dict()


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "step": step,
            "modules": modules,
            "failure_module": failure_module,
        })
        if retry is not UNSET:
            field_dict["retry"] = retry

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.completed_job_flow_status_retry import CompletedJobFlowStatusRetry
        from ..models.completed_job_flow_status_failure_module import CompletedJobFlowStatusFailureModule
        from ..models.completed_job_flow_status_modules_item import CompletedJobFlowStatusModulesItem
        d = src_dict.copy()
        step = d.pop("step")

        modules = []
        _modules = d.pop("modules")
        for modules_item_data in (_modules):
            modules_item = CompletedJobFlowStatusModulesItem.from_dict(modules_item_data)



            modules.append(modules_item)


        failure_module = CompletedJobFlowStatusFailureModule.from_dict(d.pop("failure_module"))




        _retry = d.pop("retry", UNSET)
        retry: Union[Unset, CompletedJobFlowStatusRetry]
        if isinstance(_retry,  Unset):
            retry = UNSET
        else:
            retry = CompletedJobFlowStatusRetry.from_dict(_retry)




        completed_job_flow_status = cls(
            step=step,
            modules=modules,
            failure_module=failure_module,
            retry=retry,
        )

        completed_job_flow_status.additional_properties = d
        return completed_job_flow_status

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
