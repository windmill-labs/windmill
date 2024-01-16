from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast
from typing import Dict

if TYPE_CHECKING:
  from ..models.create_input_args import CreateInputArgs





T = TypeVar("T", bound="CreateInput")


@_attrs_define
class CreateInput:
    """ 
        Attributes:
            name (str):
            args (CreateInputArgs):
     """

    name: str
    args: 'CreateInputArgs'
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.create_input_args import CreateInputArgs
        name = self.name
        args = self.args.to_dict()


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "name": name,
            "args": args,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.create_input_args import CreateInputArgs
        d = src_dict.copy()
        name = d.pop("name")

        args = CreateInputArgs.from_dict(d.pop("args"))




        create_input = cls(
            name=name,
            args=args,
        )

        create_input.additional_properties = d
        return create_input

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
