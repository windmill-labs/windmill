from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.update_resource_json_body_value import UpdateResourceJsonBodyValue
from ..types import UNSET, Unset

T = TypeVar("T", bound="UpdateResourceJsonBody")


@attr.s(auto_attribs=True)
class UpdateResourceJsonBody:
    """
    Attributes:
        path (Union[Unset, str]):
        description (Union[Unset, str]):
        value (Union[Unset, UpdateResourceJsonBodyValue]):
    """

    path: Union[Unset, str] = UNSET
    description: Union[Unset, str] = UNSET
    value: Union[Unset, UpdateResourceJsonBodyValue] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        path = self.path
        description = self.description
        value: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.value, Unset):
            value = self.value.to_dict()

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if path is not UNSET:
            field_dict["path"] = path
        if description is not UNSET:
            field_dict["description"] = description
        if value is not UNSET:
            field_dict["value"] = value

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        path = d.pop("path", UNSET)

        description = d.pop("description", UNSET)

        _value = d.pop("value", UNSET)
        value: Union[Unset, UpdateResourceJsonBodyValue]
        if isinstance(_value, Unset):
            value = UNSET
        else:
            value = UpdateResourceJsonBodyValue.from_dict(_value)

        update_resource_json_body = cls(
            path=path,
            description=description,
            value=value,
        )

        update_resource_json_body.additional_properties = d
        return update_resource_json_body

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
