from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.open_flow_value_modules_item_retry_constant import OpenFlowValueModulesItemRetryConstant
from ..models.open_flow_value_modules_item_retry_exponential import OpenFlowValueModulesItemRetryExponential
from ..types import UNSET, Unset

T = TypeVar("T", bound="OpenFlowValueModulesItemRetry")


@attr.s(auto_attribs=True)
class OpenFlowValueModulesItemRetry:
    """
    Attributes:
        constant (Union[Unset, OpenFlowValueModulesItemRetryConstant]):
        exponential (Union[Unset, OpenFlowValueModulesItemRetryExponential]):
    """

    constant: Union[Unset, OpenFlowValueModulesItemRetryConstant] = UNSET
    exponential: Union[Unset, OpenFlowValueModulesItemRetryExponential] = UNSET
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
        constant: Union[Unset, OpenFlowValueModulesItemRetryConstant]
        if isinstance(_constant, Unset):
            constant = UNSET
        else:
            constant = OpenFlowValueModulesItemRetryConstant.from_dict(_constant)

        _exponential = d.pop("exponential", UNSET)
        exponential: Union[Unset, OpenFlowValueModulesItemRetryExponential]
        if isinstance(_exponential, Unset):
            exponential = UNSET
        else:
            exponential = OpenFlowValueModulesItemRetryExponential.from_dict(_exponential)

        open_flow_value_modules_item_retry = cls(
            constant=constant,
            exponential=exponential,
        )

        open_flow_value_modules_item_retry.additional_properties = d
        return open_flow_value_modules_item_retry

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
