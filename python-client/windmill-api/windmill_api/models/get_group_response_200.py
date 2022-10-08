from typing import Any, Dict, List, Type, TypeVar, Union, cast

import attr

from ..models.get_group_response_200_extra_perms import GetGroupResponse200ExtraPerms
from ..types import UNSET, Unset

T = TypeVar("T", bound="GetGroupResponse200")


@attr.s(auto_attribs=True)
class GetGroupResponse200:
    """
    Attributes:
        name (str):
        summary (Union[Unset, str]):
        members (Union[Unset, List[str]]):
        extra_perms (Union[Unset, GetGroupResponse200ExtraPerms]):
    """

    name: str
    summary: Union[Unset, str] = UNSET
    members: Union[Unset, List[str]] = UNSET
    extra_perms: Union[Unset, GetGroupResponse200ExtraPerms] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        name = self.name
        summary = self.summary
        members: Union[Unset, List[str]] = UNSET
        if not isinstance(self.members, Unset):
            members = self.members

        extra_perms: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.extra_perms, Unset):
            extra_perms = self.extra_perms.to_dict()

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "name": name,
            }
        )
        if summary is not UNSET:
            field_dict["summary"] = summary
        if members is not UNSET:
            field_dict["members"] = members
        if extra_perms is not UNSET:
            field_dict["extra_perms"] = extra_perms

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        name = d.pop("name")

        summary = d.pop("summary", UNSET)

        members = cast(List[str], d.pop("members", UNSET))

        _extra_perms = d.pop("extra_perms", UNSET)
        extra_perms: Union[Unset, GetGroupResponse200ExtraPerms]
        if isinstance(_extra_perms, Unset):
            extra_perms = UNSET
        else:
            extra_perms = GetGroupResponse200ExtraPerms.from_dict(_extra_perms)

        get_group_response_200 = cls(
            name=name,
            summary=summary,
            members=members,
            extra_perms=extra_perms,
        )

        get_group_response_200.additional_properties = d
        return get_group_response_200

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
