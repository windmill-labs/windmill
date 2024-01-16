from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="EditAutoInviteJsonBody")


@_attrs_define
class EditAutoInviteJsonBody:
    """ 
        Attributes:
            operator (Union[Unset, bool]):
            invite_all (Union[Unset, bool]):
     """

    operator: Union[Unset, bool] = UNSET
    invite_all: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        operator = self.operator
        invite_all = self.invite_all

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if operator is not UNSET:
            field_dict["operator"] = operator
        if invite_all is not UNSET:
            field_dict["invite_all"] = invite_all

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        operator = d.pop("operator", UNSET)

        invite_all = d.pop("invite_all", UNSET)

        edit_auto_invite_json_body = cls(
            operator=operator,
            invite_all=invite_all,
        )

        edit_auto_invite_json_body.additional_properties = d
        return edit_auto_invite_json_body

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
