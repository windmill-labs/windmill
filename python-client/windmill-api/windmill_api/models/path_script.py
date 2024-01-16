from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from ..models.path_script_type import PathScriptType
from typing import Dict
from ..types import UNSET, Unset

if TYPE_CHECKING:
  from ..models.path_script_input_transforms import PathScriptInputTransforms





T = TypeVar("T", bound="PathScript")


@_attrs_define
class PathScript:
    """ 
        Attributes:
            input_transforms (PathScriptInputTransforms):
            path (str):
            type (PathScriptType):
            hash_ (Union[Unset, str]):
     """

    input_transforms: 'PathScriptInputTransforms'
    path: str
    type: PathScriptType
    hash_: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.path_script_input_transforms import PathScriptInputTransforms
        input_transforms = self.input_transforms.to_dict()

        path = self.path
        type = self.type.value

        hash_ = self.hash_

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "input_transforms": input_transforms,
            "path": path,
            "type": type,
        })
        if hash_ is not UNSET:
            field_dict["hash"] = hash_

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.path_script_input_transforms import PathScriptInputTransforms
        d = src_dict.copy()
        input_transforms = PathScriptInputTransforms.from_dict(d.pop("input_transforms"))




        path = d.pop("path")

        type = PathScriptType(d.pop("type"))




        hash_ = d.pop("hash", UNSET)

        path_script = cls(
            input_transforms=input_transforms,
            path=path,
            type=type,
            hash_=hash_,
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
