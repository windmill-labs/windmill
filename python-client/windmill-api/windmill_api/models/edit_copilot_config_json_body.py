from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="EditCopilotConfigJsonBody")


@_attrs_define
class EditCopilotConfigJsonBody:
    """ 
        Attributes:
            code_completion_enabled (bool):
            openai_resource_path (Union[Unset, str]):
     """

    code_completion_enabled: bool
    openai_resource_path: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        code_completion_enabled = self.code_completion_enabled
        openai_resource_path = self.openai_resource_path

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "code_completion_enabled": code_completion_enabled,
        })
        if openai_resource_path is not UNSET:
            field_dict["openai_resource_path"] = openai_resource_path

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        code_completion_enabled = d.pop("code_completion_enabled")

        openai_resource_path = d.pop("openai_resource_path", UNSET)

        edit_copilot_config_json_body = cls(
            code_completion_enabled=code_completion_enabled,
            openai_resource_path=openai_resource_path,
        )

        edit_copilot_config_json_body.additional_properties = d
        return edit_copilot_config_json_body

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
