from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.listable_variable_extra_perms import ListableVariableExtraPerms
from ..types import UNSET, Unset

T = TypeVar("T", bound="ListableVariable")


@attr.s(auto_attribs=True)
class ListableVariable:
    """
    Attributes:
        workspace_id (str):
        path (str):
        is_secret (bool):
        extra_perms (ListableVariableExtraPerms):
        value (Union[Unset, str]):
        description (Union[Unset, str]):
        account (Union[Unset, str]):
        is_oauth (Union[Unset, bool]):
    """

    workspace_id: str
    path: str
    is_secret: bool
    extra_perms: ListableVariableExtraPerms
    value: Union[Unset, str] = UNSET
    description: Union[Unset, str] = UNSET
    account: Union[Unset, str] = UNSET
    is_oauth: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        workspace_id = self.workspace_id
        path = self.path
        is_secret = self.is_secret
        extra_perms = self.extra_perms.to_dict()

        value = self.value
        description = self.description
        account = self.account
        is_oauth = self.is_oauth

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "workspace_id": workspace_id,
                "path": path,
                "is_secret": is_secret,
                "extra_perms": extra_perms,
            }
        )
        if value is not UNSET:
            field_dict["value"] = value
        if description is not UNSET:
            field_dict["description"] = description
        if account is not UNSET:
            field_dict["account"] = account
        if is_oauth is not UNSET:
            field_dict["is_oauth"] = is_oauth

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        workspace_id = d.pop("workspace_id")

        path = d.pop("path")

        is_secret = d.pop("is_secret")

        extra_perms = ListableVariableExtraPerms.from_dict(d.pop("extra_perms"))

        value = d.pop("value", UNSET)

        description = d.pop("description", UNSET)

        account = d.pop("account", UNSET)

        is_oauth = d.pop("is_oauth", UNSET)

        listable_variable = cls(
            workspace_id=workspace_id,
            path=path,
            is_secret=is_secret,
            extra_perms=extra_perms,
            value=value,
            description=description,
            account=account,
            is_oauth=is_oauth,
        )

        listable_variable.additional_properties = d
        return listable_variable

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
