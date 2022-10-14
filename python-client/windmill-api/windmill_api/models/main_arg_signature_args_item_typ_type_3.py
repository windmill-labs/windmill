from typing import Any, Dict, List, Type, TypeVar

import attr

from ..models.main_arg_signature_args_item_typ_type_3_object_item import MainArgSignatureArgsItemTypType3ObjectItem

T = TypeVar("T", bound="MainArgSignatureArgsItemTypType3")


@attr.s(auto_attribs=True)
class MainArgSignatureArgsItemTypType3:
    """
    Attributes:
        object_ (List[MainArgSignatureArgsItemTypType3ObjectItem]):
    """

    object_: List[MainArgSignatureArgsItemTypType3ObjectItem]
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        object_ = []
        for object_item_data in self.object_:
            object_item = object_item_data.to_dict()

            object_.append(object_item)

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "object": object_,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        object_ = []
        _object_ = d.pop("object")
        for object_item_data in _object_:
            object_item = MainArgSignatureArgsItemTypType3ObjectItem.from_dict(object_item_data)

            object_.append(object_item)

        main_arg_signature_args_item_typ_type_3 = cls(
            object_=object_,
        )

        main_arg_signature_args_item_typ_type_3.additional_properties = d
        return main_arg_signature_args_item_typ_type_3

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
