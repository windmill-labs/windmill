from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.resource_extra_perms import ResourceExtraPerms
from ..models.resource_value import ResourceValue
from ..types import UNSET, Unset

T = TypeVar("T", bound="Resource")


@attr.s(auto_attribs=True)
class Resource:
    """
    Attributes:
        path (str):
        resource_type (str):
        is_oauth (bool):
        workspace_id (Union[Unset, str]):
        description (Union[Unset, str]):
        value (Union[Unset, ResourceValue]):
        extra_perms (Union[Unset, ResourceExtraPerms]):
    """

    path: str
    resource_type: str
    is_oauth: bool
    workspace_id: Union[Unset, str] = UNSET
    description: Union[Unset, str] = UNSET
    value: Union[Unset, ResourceValue] = UNSET
    extra_perms: Union[Unset, ResourceExtraPerms] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        path = self.path
        resource_type = self.resource_type
        is_oauth = self.is_oauth
        workspace_id = self.workspace_id
        description = self.description
        value: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.value, Unset):
            value = self.value.to_dict()

        extra_perms: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.extra_perms, Unset):
            extra_perms = self.extra_perms.to_dict()

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "path": path,
                "resource_type": resource_type,
                "is_oauth": is_oauth,
            }
        )
        if workspace_id is not UNSET:
            field_dict["workspace_id"] = workspace_id
        if description is not UNSET:
            field_dict["description"] = description
        if value is not UNSET:
            field_dict["value"] = value
        if extra_perms is not UNSET:
            field_dict["extra_perms"] = extra_perms

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        path = d.pop("path")

        resource_type = d.pop("resource_type")

        is_oauth = d.pop("is_oauth")

        workspace_id = d.pop("workspace_id", UNSET)

        description = d.pop("description", UNSET)

        _value = d.pop("value", UNSET)
        value: Union[Unset, ResourceValue]
        if isinstance(_value, Unset):
            value = UNSET
        else:
            value = ResourceValue.from_dict(_value)

        _extra_perms = d.pop("extra_perms", UNSET)
        extra_perms: Union[Unset, ResourceExtraPerms]
        if isinstance(_extra_perms, Unset):
            extra_perms = UNSET
        else:
            extra_perms = ResourceExtraPerms.from_dict(_extra_perms)

        resource = cls(
            path=path,
            resource_type=resource_type,
            is_oauth=is_oauth,
            workspace_id=workspace_id,
            description=description,
            value=value,
            extra_perms=extra_perms,
        )

        resource.additional_properties = d
        return resource

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
