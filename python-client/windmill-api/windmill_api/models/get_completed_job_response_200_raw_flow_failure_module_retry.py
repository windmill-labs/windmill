from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.get_completed_job_response_200_raw_flow_failure_module_retry_constant import (
    GetCompletedJobResponse200RawFlowFailureModuleRetryConstant,
)
from ..models.get_completed_job_response_200_raw_flow_failure_module_retry_exponential import (
    GetCompletedJobResponse200RawFlowFailureModuleRetryExponential,
)
from ..types import UNSET, Unset

T = TypeVar("T", bound="GetCompletedJobResponse200RawFlowFailureModuleRetry")


@attr.s(auto_attribs=True)
class GetCompletedJobResponse200RawFlowFailureModuleRetry:
    """
    Attributes:
        constant (Union[Unset, GetCompletedJobResponse200RawFlowFailureModuleRetryConstant]):
        exponential (Union[Unset, GetCompletedJobResponse200RawFlowFailureModuleRetryExponential]):
    """

    constant: Union[Unset, GetCompletedJobResponse200RawFlowFailureModuleRetryConstant] = UNSET
    exponential: Union[Unset, GetCompletedJobResponse200RawFlowFailureModuleRetryExponential] = UNSET
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
        constant: Union[Unset, GetCompletedJobResponse200RawFlowFailureModuleRetryConstant]
        if isinstance(_constant, Unset):
            constant = UNSET
        else:
            constant = GetCompletedJobResponse200RawFlowFailureModuleRetryConstant.from_dict(_constant)

        _exponential = d.pop("exponential", UNSET)
        exponential: Union[Unset, GetCompletedJobResponse200RawFlowFailureModuleRetryExponential]
        if isinstance(_exponential, Unset):
            exponential = UNSET
        else:
            exponential = GetCompletedJobResponse200RawFlowFailureModuleRetryExponential.from_dict(_exponential)

        get_completed_job_response_200_raw_flow_failure_module_retry = cls(
            constant=constant,
            exponential=exponential,
        )

        get_completed_job_response_200_raw_flow_failure_module_retry.additional_properties = d
        return get_completed_job_response_200_raw_flow_failure_module_retry

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
