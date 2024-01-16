from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast
from typing import Dict
from typing import cast, Union
from typing import Optional
from ..models.main_arg_signature_args_item_typ_type_4_list_type_0 import MainArgSignatureArgsItemTypType4ListType0

if TYPE_CHECKING:
  from ..models.main_arg_signature_args_item_typ_type_4_list_type_1 import MainArgSignatureArgsItemTypType4ListType1





T = TypeVar("T", bound="MainArgSignatureArgsItemTypType4")


@_attrs_define
class MainArgSignatureArgsItemTypType4:
    """ 
        Attributes:
            list_ (Union['MainArgSignatureArgsItemTypType4ListType1', MainArgSignatureArgsItemTypType4ListType0, None]):
     """

    list_: Union['MainArgSignatureArgsItemTypType4ListType1', MainArgSignatureArgsItemTypType4ListType0, None]
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.main_arg_signature_args_item_typ_type_4_list_type_1 import MainArgSignatureArgsItemTypType4ListType1
        list_: Union[Dict[str, Any], None, str]
        if self.list_ is None:
            list_ = None

        elif isinstance(self.list_, MainArgSignatureArgsItemTypType4ListType0):
            list_ = self.list_.value

        else:
            list_ = self.list_.to_dict()




        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "list": list_,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.main_arg_signature_args_item_typ_type_4_list_type_1 import MainArgSignatureArgsItemTypType4ListType1
        d = src_dict.copy()
        def _parse_list_(data: object) -> Union['MainArgSignatureArgsItemTypType4ListType1', MainArgSignatureArgsItemTypType4ListType0, None]:
            if data is None:
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                list_type_0 = MainArgSignatureArgsItemTypType4ListType0(data)



                return list_type_0
            except: # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            list_type_1 = MainArgSignatureArgsItemTypType4ListType1.from_dict(data)



            return list_type_1

        list_ = _parse_list_(d.pop("list"))


        main_arg_signature_args_item_typ_type_4 = cls(
            list_=list_,
        )

        main_arg_signature_args_item_typ_type_4.additional_properties = d
        return main_arg_signature_args_item_typ_type_4

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
