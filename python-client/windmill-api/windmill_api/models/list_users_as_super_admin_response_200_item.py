from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.list_users_as_super_admin_response_200_item_login_type import (
    ListUsersAsSuperAdminResponse200ItemLoginType,
)
from ..types import UNSET, Unset

T = TypeVar("T", bound="ListUsersAsSuperAdminResponse200Item")


@attr.s(auto_attribs=True)
class ListUsersAsSuperAdminResponse200Item:
    """
    Attributes:
        email (str):
        login_type (ListUsersAsSuperAdminResponse200ItemLoginType):
        super_admin (bool):
        verified (bool):
        name (Union[Unset, str]):
        company (Union[Unset, str]):
    """

    email: str
    login_type: ListUsersAsSuperAdminResponse200ItemLoginType
    super_admin: bool
    verified: bool
    name: Union[Unset, str] = UNSET
    company: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        email = self.email
        login_type = self.login_type.value

        super_admin = self.super_admin
        verified = self.verified
        name = self.name
        company = self.company

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "email": email,
                "login_type": login_type,
                "super_admin": super_admin,
                "verified": verified,
            }
        )
        if name is not UNSET:
            field_dict["name"] = name
        if company is not UNSET:
            field_dict["company"] = company

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        email = d.pop("email")

        login_type = ListUsersAsSuperAdminResponse200ItemLoginType(d.pop("login_type"))

        super_admin = d.pop("super_admin")

        verified = d.pop("verified")

        name = d.pop("name", UNSET)

        company = d.pop("company", UNSET)

        list_users_as_super_admin_response_200_item = cls(
            email=email,
            login_type=login_type,
            super_admin=super_admin,
            verified=verified,
            name=name,
            company=company,
        )

        list_users_as_super_admin_response_200_item.additional_properties = d
        return list_users_as_super_admin_response_200_item

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
