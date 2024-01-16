from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="ExecuteComponentJsonBodyRawCode")


@_attrs_define
class ExecuteComponentJsonBodyRawCode:
    """ 
        Attributes:
            content (str):
            language (str):
            path (Union[Unset, str]):
            cache_ttl (Union[Unset, int]):
     """

    content: str
    language: str
    path: Union[Unset, str] = UNSET
    cache_ttl: Union[Unset, int] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        content = self.content
        language = self.language
        path = self.path
        cache_ttl = self.cache_ttl

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "content": content,
            "language": language,
        })
        if path is not UNSET:
            field_dict["path"] = path
        if cache_ttl is not UNSET:
            field_dict["cache_ttl"] = cache_ttl

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        content = d.pop("content")

        language = d.pop("language")

        path = d.pop("path", UNSET)

        cache_ttl = d.pop("cache_ttl", UNSET)

        execute_component_json_body_raw_code = cls(
            content=content,
            language=language,
            path=path,
            cache_ttl=cache_ttl,
        )

        execute_component_json_body_raw_code.additional_properties = d
        return execute_component_json_body_raw_code

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
