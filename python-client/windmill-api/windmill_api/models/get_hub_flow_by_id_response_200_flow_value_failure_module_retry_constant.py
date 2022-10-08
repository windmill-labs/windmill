from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..types import UNSET, Unset

T = TypeVar("T", bound="GetHubFlowByIdResponse200FlowValueFailureModuleRetryConstant")


@attr.s(auto_attribs=True)
class GetHubFlowByIdResponse200FlowValueFailureModuleRetryConstant:
    """
    Attributes:
        attempts (Union[Unset, int]):
        seconds (Union[Unset, int]):
    """

    attempts: Union[Unset, int] = UNSET
    seconds: Union[Unset, int] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        attempts = self.attempts
        seconds = self.seconds

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if attempts is not UNSET:
            field_dict["attempts"] = attempts
        if seconds is not UNSET:
            field_dict["seconds"] = seconds

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        attempts = d.pop("attempts", UNSET)

        seconds = d.pop("seconds", UNSET)

        get_hub_flow_by_id_response_200_flow_value_failure_module_retry_constant = cls(
            attempts=attempts,
            seconds=seconds,
        )

        get_hub_flow_by_id_response_200_flow_value_failure_module_retry_constant.additional_properties = d
        return get_hub_flow_by_id_response_200_flow_value_failure_module_retry_constant

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
