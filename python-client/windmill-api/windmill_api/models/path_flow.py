from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.path_flow_type import PathFlowType
from ..types import UNSET, Unset

T = TypeVar("T", bound="PathFlow")


@attr.s(auto_attribs=True)
class PathFlow:
    """
    Attributes:
        type (PathFlowType):
        path (Union[Unset, str]):
    """

    type: PathFlowType
    path: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        type = self.type.value

        path = self.path

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "type": type,
            }
        )
        if path is not UNSET:
            field_dict["path"] = path

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        type = PathFlowType(d.pop("type"))

        path = d.pop("path", UNSET)

        path_flow = cls(
            type=type,
            path=path,
        )

        path_flow.additional_properties = d
        return path_flow

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
