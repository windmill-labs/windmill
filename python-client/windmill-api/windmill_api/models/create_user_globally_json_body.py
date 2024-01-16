from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="CreateUserGloballyJsonBody")


@_attrs_define
class CreateUserGloballyJsonBody:
    """ 
        Attributes:
            email (str):
            password (str):
            super_admin (bool):
            name (Union[Unset, str]):
            company (Union[Unset, str]):
     """

    email: str
    password: str
    super_admin: bool
    name: Union[Unset, str] = UNSET
    company: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        email = self.email
        password = self.password
        super_admin = self.super_admin
        name = self.name
        company = self.company

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "email": email,
            "password": password,
            "super_admin": super_admin,
        })
        if name is not UNSET:
            field_dict["name"] = name
        if company is not UNSET:
            field_dict["company"] = company

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        email = d.pop("email")

        password = d.pop("password")

        super_admin = d.pop("super_admin")

        name = d.pop("name", UNSET)

        company = d.pop("company", UNSET)

        create_user_globally_json_body = cls(
            email=email,
            password=password,
            super_admin=super_admin,
            name=name,
            company=company,
        )

        create_user_globally_json_body.additional_properties = d
        return create_user_globally_json_body

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
