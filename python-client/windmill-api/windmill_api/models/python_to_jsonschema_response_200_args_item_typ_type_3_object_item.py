from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.python_to_jsonschema_response_200_args_item_typ_type_3_object_item_typ_type_0 import (
    PythonToJsonschemaResponse200ArgsItemTypType3ObjectItemTypType0,
)
from ..models.python_to_jsonschema_response_200_args_item_typ_type_3_object_item_typ_type_1 import (
    PythonToJsonschemaResponse200ArgsItemTypType3ObjectItemTypType1,
)

T = TypeVar("T", bound="PythonToJsonschemaResponse200ArgsItemTypType3ObjectItem")


@attr.s(auto_attribs=True)
class PythonToJsonschemaResponse200ArgsItemTypType3ObjectItem:
    """
    Attributes:
        key (str):
        typ (Union[PythonToJsonschemaResponse200ArgsItemTypType3ObjectItemTypType0,
            PythonToJsonschemaResponse200ArgsItemTypType3ObjectItemTypType1]):
    """

    key: str
    typ: Union[
        PythonToJsonschemaResponse200ArgsItemTypType3ObjectItemTypType0,
        PythonToJsonschemaResponse200ArgsItemTypType3ObjectItemTypType1,
    ]
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        key = self.key

        if isinstance(self.typ, PythonToJsonschemaResponse200ArgsItemTypType3ObjectItemTypType0):
            typ = self.typ.value

        else:
            typ = self.typ.to_dict()

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "key": key,
                "typ": typ,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        key = d.pop("key")

        def _parse_typ(
            data: object,
        ) -> Union[
            PythonToJsonschemaResponse200ArgsItemTypType3ObjectItemTypType0,
            PythonToJsonschemaResponse200ArgsItemTypType3ObjectItemTypType1,
        ]:
            try:
                if not isinstance(data, str):
                    raise TypeError()
                typ_type_0 = PythonToJsonschemaResponse200ArgsItemTypType3ObjectItemTypType0(data)

                return typ_type_0
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            typ_type_1 = PythonToJsonschemaResponse200ArgsItemTypType3ObjectItemTypType1.from_dict(data)

            return typ_type_1

        typ = _parse_typ(d.pop("typ"))

        python_to_jsonschema_response_200_args_item_typ_type_3_object_item = cls(
            key=key,
            typ=typ,
        )

        python_to_jsonschema_response_200_args_item_typ_type_3_object_item.additional_properties = d
        return python_to_jsonschema_response_200_args_item_typ_type_3_object_item

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
