from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset
from ..models.flow_module_value_2_type_6_type import FlowModuleValue2Type6Type






T = TypeVar("T", bound="FlowModuleValue2Type6")


@_attrs_define
class FlowModuleValue2Type6:
    """ 
        Attributes:
            type (FlowModuleValue2Type6Type):
            flow (Union[Unset, bool]):
     """

    type: FlowModuleValue2Type6Type
    flow: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        type = self.type.value

        flow = self.flow

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "type": type,
        })
        if flow is not UNSET:
            field_dict["flow"] = flow

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        type = FlowModuleValue2Type6Type(d.pop("type"))




        flow = d.pop("flow", UNSET)

        flow_module_value_2_type_6 = cls(
            type=type,
            flow=flow,
        )

        flow_module_value_2_type_6.additional_properties = d
        return flow_module_value_2_type_6

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
