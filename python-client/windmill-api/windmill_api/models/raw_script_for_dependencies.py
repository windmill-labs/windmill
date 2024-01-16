from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from ..models.raw_script_for_dependencies_language import RawScriptForDependenciesLanguage






T = TypeVar("T", bound="RawScriptForDependencies")


@_attrs_define
class RawScriptForDependencies:
    """ 
        Attributes:
            content (str):
            path (str):
            language (RawScriptForDependenciesLanguage):
     """

    content: str
    path: str
    language: RawScriptForDependenciesLanguage
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        content = self.content
        path = self.path
        language = self.language.value


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "content": content,
            "path": path,
            "language": language,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        content = d.pop("content")

        path = d.pop("path")

        language = RawScriptForDependenciesLanguage(d.pop("language"))




        raw_script_for_dependencies = cls(
            content=content,
            path=path,
            language=language,
        )

        raw_script_for_dependencies.additional_properties = d
        return raw_script_for_dependencies

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
