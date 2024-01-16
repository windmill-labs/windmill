from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="ScriptHistory")


@_attrs_define
class ScriptHistory:
    """ 
        Attributes:
            script_hash (str):
            deployment_msg (Union[Unset, str]):
     """

    script_hash: str
    deployment_msg: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        script_hash = self.script_hash
        deployment_msg = self.deployment_msg

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "script_hash": script_hash,
        })
        if deployment_msg is not UNSET:
            field_dict["deployment_msg"] = deployment_msg

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        script_hash = d.pop("script_hash")

        deployment_msg = d.pop("deployment_msg", UNSET)

        script_history = cls(
            script_hash=script_hash,
            deployment_msg=deployment_msg,
        )

        script_history.additional_properties = d
        return script_history

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
