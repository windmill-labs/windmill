from typing import Any, Dict, List, Type, TypeVar

import attr

from ..models.path_script_type import PathScriptType

T = TypeVar("T", bound="PathScript")


@attr.s(auto_attribs=True)
class PathScript:
    """
    Attributes:
        path (str):
        type (PathScriptType):
    """

    path: str
    type: PathScriptType
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        path = self.path
        type = self.type.value

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "path": path,
                "type": type,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        path = d.pop("path")

        type = PathScriptType(d.pop("type"))

        path_script = cls(
            path=path,
            type=type,
        )

        path_script.additional_properties = d
        return path_script

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
