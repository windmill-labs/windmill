from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="ToggleWorkspaceErrorHandlerForFlowJsonBody")


@_attrs_define
class ToggleWorkspaceErrorHandlerForFlowJsonBody:
    """ 
        Attributes:
            muted (Union[Unset, bool]):
     """

    muted: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        muted = self.muted

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if muted is not UNSET:
            field_dict["muted"] = muted

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        muted = d.pop("muted", UNSET)

        toggle_workspace_error_handler_for_flow_json_body = cls(
            muted=muted,
        )

        toggle_workspace_error_handler_for_flow_json_body.additional_properties = d
        return toggle_workspace_error_handler_for_flow_json_body

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
