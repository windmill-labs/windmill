from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from ..types import UNSET, Unset
from typing import Dict

if TYPE_CHECKING:
  from ..models.get_variable_response_200_extra_perms import GetVariableResponse200ExtraPerms





T = TypeVar("T", bound="GetVariableResponse200")


@_attrs_define
class GetVariableResponse200:
    """ 
        Attributes:
            workspace_id (str):
            path (str):
            is_secret (bool):
            extra_perms (GetVariableResponse200ExtraPerms):
            value (Union[Unset, str]):
            description (Union[Unset, str]):
            account (Union[Unset, int]):
            is_oauth (Union[Unset, bool]):
            is_expired (Union[Unset, bool]):
            refresh_error (Union[Unset, str]):
            is_linked (Union[Unset, bool]):
            is_refreshed (Union[Unset, bool]):
     """

    workspace_id: str
    path: str
    is_secret: bool
    extra_perms: 'GetVariableResponse200ExtraPerms'
    value: Union[Unset, str] = UNSET
    description: Union[Unset, str] = UNSET
    account: Union[Unset, int] = UNSET
    is_oauth: Union[Unset, bool] = UNSET
    is_expired: Union[Unset, bool] = UNSET
    refresh_error: Union[Unset, str] = UNSET
    is_linked: Union[Unset, bool] = UNSET
    is_refreshed: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.get_variable_response_200_extra_perms import GetVariableResponse200ExtraPerms
        workspace_id = self.workspace_id
        path = self.path
        is_secret = self.is_secret
        extra_perms = self.extra_perms.to_dict()

        value = self.value
        description = self.description
        account = self.account
        is_oauth = self.is_oauth
        is_expired = self.is_expired
        refresh_error = self.refresh_error
        is_linked = self.is_linked
        is_refreshed = self.is_refreshed

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "workspace_id": workspace_id,
            "path": path,
            "is_secret": is_secret,
            "extra_perms": extra_perms,
        })
        if value is not UNSET:
            field_dict["value"] = value
        if description is not UNSET:
            field_dict["description"] = description
        if account is not UNSET:
            field_dict["account"] = account
        if is_oauth is not UNSET:
            field_dict["is_oauth"] = is_oauth
        if is_expired is not UNSET:
            field_dict["is_expired"] = is_expired
        if refresh_error is not UNSET:
            field_dict["refresh_error"] = refresh_error
        if is_linked is not UNSET:
            field_dict["is_linked"] = is_linked
        if is_refreshed is not UNSET:
            field_dict["is_refreshed"] = is_refreshed

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.get_variable_response_200_extra_perms import GetVariableResponse200ExtraPerms
        d = src_dict.copy()
        workspace_id = d.pop("workspace_id")

        path = d.pop("path")

        is_secret = d.pop("is_secret")

        extra_perms = GetVariableResponse200ExtraPerms.from_dict(d.pop("extra_perms"))




        value = d.pop("value", UNSET)

        description = d.pop("description", UNSET)

        account = d.pop("account", UNSET)

        is_oauth = d.pop("is_oauth", UNSET)

        is_expired = d.pop("is_expired", UNSET)

        refresh_error = d.pop("refresh_error", UNSET)

        is_linked = d.pop("is_linked", UNSET)

        is_refreshed = d.pop("is_refreshed", UNSET)

        get_variable_response_200 = cls(
            workspace_id=workspace_id,
            path=path,
            is_secret=is_secret,
            extra_perms=extra_perms,
            value=value,
            description=description,
            account=account,
            is_oauth=is_oauth,
            is_expired=is_expired,
            refresh_error=refresh_error,
            is_linked=is_linked,
            is_refreshed=is_refreshed,
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
