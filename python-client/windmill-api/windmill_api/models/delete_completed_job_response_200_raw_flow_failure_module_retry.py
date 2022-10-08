from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.delete_completed_job_response_200_raw_flow_failure_module_retry_constant import (
    DeleteCompletedJobResponse200RawFlowFailureModuleRetryConstant,
)
from ..models.delete_completed_job_response_200_raw_flow_failure_module_retry_exponential import (
    DeleteCompletedJobResponse200RawFlowFailureModuleRetryExponential,
)
from ..types import UNSET, Unset

T = TypeVar("T", bound="DeleteCompletedJobResponse200RawFlowFailureModuleRetry")


@attr.s(auto_attribs=True)
class DeleteCompletedJobResponse200RawFlowFailureModuleRetry:
    """
    Attributes:
        constant (Union[Unset, DeleteCompletedJobResponse200RawFlowFailureModuleRetryConstant]):
        exponential (Union[Unset, DeleteCompletedJobResponse200RawFlowFailureModuleRetryExponential]):
    """

    constant: Union[Unset, DeleteCompletedJobResponse200RawFlowFailureModuleRetryConstant] = UNSET
    exponential: Union[Unset, DeleteCompletedJobResponse200RawFlowFailureModuleRetryExponential] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        constant: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.constant, Unset):
            constant = self.constant.to_dict()

        exponential: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.exponential, Unset):
            exponential = self.exponential.to_dict()

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if constant is not UNSET:
            field_dict["constant"] = constant
        if exponential is not UNSET:
            field_dict["exponential"] = exponential

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        _constant = d.pop("constant", UNSET)
        constant: Union[Unset, DeleteCompletedJobResponse200RawFlowFailureModuleRetryConstant]
        if isinstance(_constant, Unset):
            constant = UNSET
        else:
            constant = DeleteCompletedJobResponse200RawFlowFailureModuleRetryConstant.from_dict(_constant)

        _exponential = d.pop("exponential", UNSET)
        exponential: Union[Unset, DeleteCompletedJobResponse200RawFlowFailureModuleRetryExponential]
        if isinstance(_exponential, Unset):
            exponential = UNSET
        else:
            exponential = DeleteCompletedJobResponse200RawFlowFailureModuleRetryExponential.from_dict(_exponential)

        delete_completed_job_response_200_raw_flow_failure_module_retry = cls(
            constant=constant,
            exponential=exponential,
        )

        delete_completed_job_response_200_raw_flow_failure_module_retry.additional_properties = d
        return delete_completed_job_response_200_raw_flow_failure_module_retry

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
