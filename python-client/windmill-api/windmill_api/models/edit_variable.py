from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..types import UNSET, Unset

T = TypeVar("T", bound="EditVariable")


@attr.s(auto_attribs=True)
class EditVariable:
    """
    Attributes:
        path (Union[Unset, str]):
        value (Union[Unset, str]):
        is_secret (Union[Unset, bool]):
        description (Union[Unset, str]):
    """

    path: Union[Unset, str] = UNSET
    value: Union[Unset, str] = UNSET
    is_secret: Union[Unset, bool] = UNSET
    description: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        path = self.path
        value = self.value
        is_secret = self.is_secret
        description = self.description

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if path is not UNSET:
            field_dict["path"] = path
        if value is not UNSET:
            field_dict["value"] = value
        if is_secret is not UNSET:
            field_dict["is_secret"] = is_secret
        if description is not UNSET:
            field_dict["description"] = description

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        path = d.pop("path", UNSET)

        value = d.pop("value", UNSET)

        is_secret = d.pop("is_secret", UNSET)

        description = d.pop("description", UNSET)

        edit_variable = cls(
            path=path,
            value=value,
            is_secret=is_secret,
            description=description,
        )

        edit_variable.additional_properties = d
        return edit_variable

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
