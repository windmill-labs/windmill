from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from ..models.flow_module_value_2_type_7_type import FlowModuleValue2Type7Type






T = TypeVar("T", bound="FlowModuleValue2Type7")


@_attrs_define
class FlowModuleValue2Type7:
    """ 
        Attributes:
            type (FlowModuleValue2Type7Type):
     """

    type: FlowModuleValue2Type7Type
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        type = self.type.value


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "type": type,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        type = FlowModuleValue2Type7Type(d.pop("type"))




        flow_module_value_2_type_7 = cls(
            type=type,
        )

        flow_module_value_2_type_7.additional_properties = d
        return flow_module_value_2_type_7

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
