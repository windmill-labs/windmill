from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from ..models.run_raw_script_dependencies_json_body_raw_scripts_item_language import RunRawScriptDependenciesJsonBodyRawScriptsItemLanguage






T = TypeVar("T", bound="RunRawScriptDependenciesJsonBodyRawScriptsItem")


@_attrs_define
class RunRawScriptDependenciesJsonBodyRawScriptsItem:
    """ 
        Attributes:
            content (str):
            path (str):
            language (RunRawScriptDependenciesJsonBodyRawScriptsItemLanguage):
     """

    content: str
    path: str
    language: RunRawScriptDependenciesJsonBodyRawScriptsItemLanguage
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

        language = RunRawScriptDependenciesJsonBodyRawScriptsItemLanguage(d.pop("language"))




        run_raw_script_dependencies_json_body_raw_scripts_item = cls(
            content=content,
            path=path,
            language=language,
        )

        run_raw_script_dependencies_json_body_raw_scripts_item.additional_properties = d
        return run_raw_script_dependencies_json_body_raw_scripts_item

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
