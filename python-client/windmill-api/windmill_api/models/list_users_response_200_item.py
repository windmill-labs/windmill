import datetime
from typing import Any, Dict, List, Type, TypeVar, Union, cast

import attr
from dateutil.parser import isoparse

from ..models.list_users_response_200_item_usage import ListUsersResponse200ItemUsage
from ..types import UNSET, Unset

T = TypeVar("T", bound="ListUsersResponse200Item")


@attr.s(auto_attribs=True)
class ListUsersResponse200Item:
    """
    Attributes:
        email (str):
        username (str):
        is_admin (bool):
        is_super_admin (bool):
        created_at (datetime.datetime):
        operator (bool):
        disabled (bool):
        groups (Union[Unset, List[str]]):
        usage (Union[Unset, ListUsersResponse200ItemUsage]):
    """

    email: str
    username: str
    is_admin: bool
    is_super_admin: bool
    created_at: datetime.datetime
    operator: bool
    disabled: bool
    groups: Union[Unset, List[str]] = UNSET
    usage: Union[Unset, ListUsersResponse200ItemUsage] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        email = self.email
        username = self.username
        is_admin = self.is_admin
        is_super_admin = self.is_super_admin
        created_at = self.created_at.isoformat()

        operator = self.operator
        disabled = self.disabled
        groups: Union[Unset, List[str]] = UNSET
        if not isinstance(self.groups, Unset):
            groups = self.groups

        usage: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.usage, Unset):
            usage = self.usage.to_dict()

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "email": email,
                "username": username,
                "is_admin": is_admin,
                "is_super_admin": is_super_admin,
                "created_at": created_at,
                "operator": operator,
                "disabled": disabled,
            }
        )
        if groups is not UNSET:
            field_dict["groups"] = groups
        if usage is not UNSET:
            field_dict["usage"] = usage

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        email = d.pop("email")

        username = d.pop("username")

        is_admin = d.pop("is_admin")

        is_super_admin = d.pop("is_super_admin")

        created_at = isoparse(d.pop("created_at"))

        operator = d.pop("operator")

        disabled = d.pop("disabled")

        groups = cast(List[str], d.pop("groups", UNSET))

        _usage = d.pop("usage", UNSET)
        usage: Union[Unset, ListUsersResponse200ItemUsage]
        if isinstance(_usage, Unset):
            usage = UNSET
        else:
            usage = ListUsersResponse200ItemUsage.from_dict(_usage)

        list_users_response_200_item = cls(
            email=email,
            username=username,
            is_admin=is_admin,
            is_super_admin=is_super_admin,
            created_at=created_at,
            operator=operator,
            disabled=disabled,
            groups=groups,
            usage=usage,
        )

        list_users_response_200_item.additional_properties = d
        return list_users_response_200_item

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
