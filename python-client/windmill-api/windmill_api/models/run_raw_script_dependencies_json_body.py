from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast
from typing import Dict
from typing import cast, List

if TYPE_CHECKING:
  from ..models.run_raw_script_dependencies_json_body_raw_scripts_item import RunRawScriptDependenciesJsonBodyRawScriptsItem





T = TypeVar("T", bound="RunRawScriptDependenciesJsonBody")


@_attrs_define
class RunRawScriptDependenciesJsonBody:
    """ 
        Attributes:
            raw_scripts (List['RunRawScriptDependenciesJsonBodyRawScriptsItem']):
            entrypoint (str):
     """

    raw_scripts: List['RunRawScriptDependenciesJsonBodyRawScriptsItem']
    entrypoint: str
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.run_raw_script_dependencies_json_body_raw_scripts_item import RunRawScriptDependenciesJsonBodyRawScriptsItem
        raw_scripts = []
        for raw_scripts_item_data in self.raw_scripts:
            raw_scripts_item = raw_scripts_item_data.to_dict()

            raw_scripts.append(raw_scripts_item)




        entrypoint = self.entrypoint

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "raw_scripts": raw_scripts,
            "entrypoint": entrypoint,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.run_raw_script_dependencies_json_body_raw_scripts_item import RunRawScriptDependenciesJsonBodyRawScriptsItem
        d = src_dict.copy()
        raw_scripts = []
        _raw_scripts = d.pop("raw_scripts")
        for raw_scripts_item_data in (_raw_scripts):
            raw_scripts_item = RunRawScriptDependenciesJsonBodyRawScriptsItem.from_dict(raw_scripts_item_data)



            raw_scripts.append(raw_scripts_item)


        entrypoint = d.pop("entrypoint")

        run_raw_script_dependencies_json_body = cls(
            raw_scripts=raw_scripts,
            entrypoint=entrypoint,
        )

        run_raw_script_dependencies_json_body.additional_properties = d
        return run_raw_script_dependencies_json_body

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
