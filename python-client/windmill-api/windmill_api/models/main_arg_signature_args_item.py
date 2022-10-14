from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.main_arg_signature_args_item_typ_type_0 import MainArgSignatureArgsItemTypType0
from ..models.main_arg_signature_args_item_typ_type_1 import MainArgSignatureArgsItemTypType1
from ..models.main_arg_signature_args_item_typ_type_2 import MainArgSignatureArgsItemTypType2
from ..models.main_arg_signature_args_item_typ_type_3 import MainArgSignatureArgsItemTypType3
from ..models.main_arg_signature_args_item_typ_type_4 import MainArgSignatureArgsItemTypType4
from ..types import UNSET, Unset

T = TypeVar("T", bound="MainArgSignatureArgsItem")


@attr.s(auto_attribs=True)
class MainArgSignatureArgsItem:
    """
    Attributes:
        name (str):
        typ (Union[MainArgSignatureArgsItemTypType0, MainArgSignatureArgsItemTypType1, MainArgSignatureArgsItemTypType2,
            MainArgSignatureArgsItemTypType3, MainArgSignatureArgsItemTypType4]):
        has_default (Union[Unset, bool]):
        default (Union[Unset, Any]):
    """

    name: str
    typ: Union[
        MainArgSignatureArgsItemTypType0,
        MainArgSignatureArgsItemTypType1,
        MainArgSignatureArgsItemTypType2,
        MainArgSignatureArgsItemTypType3,
        MainArgSignatureArgsItemTypType4,
    ]
    has_default: Union[Unset, bool] = UNSET
    default: Union[Unset, Any] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        name = self.name

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
        field_dict.update(
            {
                "name": name,
                "typ": typ,
            }
        )
        if has_default is not UNSET:
            field_dict["has_default"] = has_default
        if default is not UNSET:
            field_dict["default"] = default

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        name = d.pop("name")

        def _parse_typ(
            data: object,
        ) -> Union[
            MainArgSignatureArgsItemTypType0,
            MainArgSignatureArgsItemTypType1,
            MainArgSignatureArgsItemTypType2,
            MainArgSignatureArgsItemTypType3,
            MainArgSignatureArgsItemTypType4,
        ]:
            try:
                if not isinstance(data, str):
                    raise TypeError()
                typ_type_0 = MainArgSignatureArgsItemTypType0(data)

                return typ_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                typ_type_1 = MainArgSignatureArgsItemTypType1.from_dict(data)

                return typ_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                typ_type_2 = MainArgSignatureArgsItemTypType2.from_dict(data)

                return typ_type_2
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                typ_type_3 = MainArgSignatureArgsItemTypType3.from_dict(data)

                return typ_type_3
            except:  # noqa: E722
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
