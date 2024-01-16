from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from typing import Dict
from typing import cast, Union
from ..models.main_arg_signature_args_item_typ_type_0 import MainArgSignatureArgsItemTypType0
from ..types import UNSET, Unset

if TYPE_CHECKING:
  from ..models.main_arg_signature_args_item_typ_type_3 import MainArgSignatureArgsItemTypType3
  from ..models.main_arg_signature_args_item_typ_type_1 import MainArgSignatureArgsItemTypType1
  from ..models.main_arg_signature_args_item_typ_type_4 import MainArgSignatureArgsItemTypType4
  from ..models.main_arg_signature_args_item_typ_type_2 import MainArgSignatureArgsItemTypType2





T = TypeVar("T", bound="MainArgSignatureArgsItem")


@_attrs_define
class MainArgSignatureArgsItem:
    """ 
        Attributes:
            name (str):
            typ (Union['MainArgSignatureArgsItemTypType1', 'MainArgSignatureArgsItemTypType2',
                'MainArgSignatureArgsItemTypType3', 'MainArgSignatureArgsItemTypType4', MainArgSignatureArgsItemTypType0]):
            has_default (Union[Unset, bool]):
            default (Union[Unset, Any]):
     """

    name: str
    typ: Union['MainArgSignatureArgsItemTypType1', 'MainArgSignatureArgsItemTypType2', 'MainArgSignatureArgsItemTypType3', 'MainArgSignatureArgsItemTypType4', MainArgSignatureArgsItemTypType0]
    has_default: Union[Unset, bool] = UNSET
    default: Union[Unset, Any] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.main_arg_signature_args_item_typ_type_3 import MainArgSignatureArgsItemTypType3
        from ..models.main_arg_signature_args_item_typ_type_1 import MainArgSignatureArgsItemTypType1
        from ..models.main_arg_signature_args_item_typ_type_4 import MainArgSignatureArgsItemTypType4
        from ..models.main_arg_signature_args_item_typ_type_2 import MainArgSignatureArgsItemTypType2
        name = self.name
        typ: Union[Dict[str, Any], str]

        if isinstance(self.typ, MainArgSignatureArgsItemTypType0):
            typ = self.typ.value

        elif isinstance(self.typ, MainArgSignatureArgsItemTypType1):
            typ = self.typ.to_dict()

        elif isinstance(self.typ, MainArgSignatureArgsItemTypType2):
            typ = self.typ.to_dict()

        elif isinstance(self.typ, MainArgSignatureArgsItemTypType3):
            typ = self.typ.to_dict()

        else:
            typ = self.typ.to_dict()



        has_default = self.has_default
        default = self.default

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "name": name,
            "typ": typ,
        })
        if has_default is not UNSET:
            field_dict["has_default"] = has_default
        if default is not UNSET:
            field_dict["default"] = default

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.main_arg_signature_args_item_typ_type_3 import MainArgSignatureArgsItemTypType3
        from ..models.main_arg_signature_args_item_typ_type_1 import MainArgSignatureArgsItemTypType1
        from ..models.main_arg_signature_args_item_typ_type_4 import MainArgSignatureArgsItemTypType4
        from ..models.main_arg_signature_args_item_typ_type_2 import MainArgSignatureArgsItemTypType2
        d = src_dict.copy()
        name = d.pop("name")

        def _parse_typ(data: object) -> Union['MainArgSignatureArgsItemTypType1', 'MainArgSignatureArgsItemTypType2', 'MainArgSignatureArgsItemTypType3', 'MainArgSignatureArgsItemTypType4', MainArgSignatureArgsItemTypType0]:
            try:
                if not isinstance(data, str):
                    raise TypeError()
                typ_type_0 = MainArgSignatureArgsItemTypType0(data)



                return typ_type_0
            except: # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                typ_type_1 = MainArgSignatureArgsItemTypType1.from_dict(data)



                return typ_type_1
            except: # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                typ_type_2 = MainArgSignatureArgsItemTypType2.from_dict(data)



                return typ_type_2
            except: # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                typ_type_3 = MainArgSignatureArgsItemTypType3.from_dict(data)



                return typ_type_3
            except: # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            typ_type_4 = MainArgSignatureArgsItemTypType4.from_dict(data)



            return typ_type_4

        typ = _parse_typ(d.pop("typ"))


        has_default = d.pop("has_default", UNSET)

        default = d.pop("default", UNSET)

        main_arg_signature_args_item = cls(
            name=name,
            typ=typ,
            has_default=has_default,
            default=default,
        )

        main_arg_signature_args_item.additional_properties = d
        return main_arg_signature_args_item

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
