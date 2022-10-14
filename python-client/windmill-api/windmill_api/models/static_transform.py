from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.static_transform_type import StaticTransformType
from ..types import UNSET, Unset

T = TypeVar("T", bound="StaticTransform")


@attr.s(auto_attribs=True)
class StaticTransform:
    """
    Attributes:
        type (StaticTransformType):
        value (Union[Unset, Any]):
    """

    type: StaticTransformType
    value: Union[Unset, Any] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        type = self.type.value

        value = self.value

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "type": type,
            }
        )
        if value is not UNSET:
            field_dict["value"] = value

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        type = StaticTransformType(d.pop("type"))

        value = d.pop("value", UNSET)

        static_transform = cls(
            type=type,
            value=value,
        )

        static_transform.additional_properties = d
        return static_transform

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
