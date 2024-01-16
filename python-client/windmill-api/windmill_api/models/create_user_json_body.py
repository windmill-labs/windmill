from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset







T = TypeVar("T", bound="CreateUserJsonBody")


@_attrs_define
class CreateUserJsonBody:
    """ 
        Attributes:
            email (str):
            username (str):
            is_admin (bool):
     """

    email: str
    username: str
    is_admin: bool
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        email = self.email
        username = self.username
        is_admin = self.is_admin

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "email": email,
            "username": username,
            "is_admin": is_admin,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        email = d.pop("email")

        username = d.pop("username")

        is_admin = d.pop("is_admin")

        create_user_json_body = cls(
            email=email,
            username=username,
            is_admin=is_admin,
        )

        create_user_json_body.additional_properties = d
        return create_user_json_body

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
