from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.get_variable_response_200_extra_perms import GetVariableResponse200ExtraPerms
from ..types import UNSET, Unset

T = TypeVar("T", bound="GetVariableResponse200")


@attr.s(auto_attribs=True)
class GetVariableResponse200:
    """
    Attributes:
        workspace_id (str):
        path (str):
        is_secret (bool):
        extra_perms (GetVariableResponse200ExtraPerms):
        value (Union[Unset, str]):
        description (Union[Unset, str]):
        account (Union[Unset, str]):
        is_oauth (Union[Unset, bool]):
    """

    workspace_id: str
    path: str
    is_secret: bool
    extra_perms: GetVariableResponse200ExtraPerms
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

        extra_perms = GetVariableResponse200ExtraPerms.from_dict(d.pop("extra_perms"))

        value = d.pop("value", UNSET)

        description = d.pop("description", UNSET)

        account = d.pop("account", UNSET)

        is_oauth = d.pop("is_oauth", UNSET)

        get_variable_response_200 = cls(
            workspace_id=workspace_id,
            path=path,
            is_secret=is_secret,
            extra_perms=extra_perms,
            value=value,
            description=description,
            account=account,
            is_oauth=is_oauth,
        )

        get_variable_response_200.additional_properties = d
        return get_variable_response_200

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
