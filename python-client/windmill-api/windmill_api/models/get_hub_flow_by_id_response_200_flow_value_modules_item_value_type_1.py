from typing import Any, Dict, List, Type, TypeVar

import attr

from ..models.get_hub_flow_by_id_response_200_flow_value_modules_item_value_type_1_type import (
    GetHubFlowByIdResponse200FlowValueModulesItemValueType1Type,
)

T = TypeVar("T", bound="GetHubFlowByIdResponse200FlowValueModulesItemValueType1")


@attr.s(auto_attribs=True)
class GetHubFlowByIdResponse200FlowValueModulesItemValueType1:
    """
    Attributes:
        path (str):
        type (GetHubFlowByIdResponse200FlowValueModulesItemValueType1Type):
    """

    path: str
    type: GetHubFlowByIdResponse200FlowValueModulesItemValueType1Type
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        path = self.path
        type = self.type.value

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "path": path,
                "type": type,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        path = d.pop("path")

        type = GetHubFlowByIdResponse200FlowValueModulesItemValueType1Type(d.pop("type"))

        get_hub_flow_by_id_response_200_flow_value_modules_item_value_type_1 = cls(
            path=path,
            type=type,
        )

        get_hub_flow_by_id_response_200_flow_value_modules_item_value_type_1.additional_properties = d
        return get_hub_flow_by_id_response_200_flow_value_modules_item_value_type_1

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
