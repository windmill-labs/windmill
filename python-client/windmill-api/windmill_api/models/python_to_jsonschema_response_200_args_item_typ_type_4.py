from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.python_to_jsonschema_response_200_args_item_typ_type_4_list_type_0 import (
    PythonToJsonschemaResponse200ArgsItemTypType4ListType0,
)
from ..models.python_to_jsonschema_response_200_args_item_typ_type_4_list_type_1 import (
    PythonToJsonschemaResponse200ArgsItemTypType4ListType1,
)

T = TypeVar("T", bound="PythonToJsonschemaResponse200ArgsItemTypType4")


@attr.s(auto_attribs=True)
class PythonToJsonschemaResponse200ArgsItemTypType4:
    """
    Attributes:
        list_ (Union[None, PythonToJsonschemaResponse200ArgsItemTypType4ListType0,
            PythonToJsonschemaResponse200ArgsItemTypType4ListType1]):
    """

    list_: Union[
        None,
        PythonToJsonschemaResponse200ArgsItemTypType4ListType0,
        PythonToJsonschemaResponse200ArgsItemTypType4ListType1,
    ]
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        list_: Union[Dict[str, Any], None, str]
        if self.list_ is None:
            list_ = None

        elif isinstance(self.list_, PythonToJsonschemaResponse200ArgsItemTypType4ListType0):
            list_ = self.list_.value

        else:
            list_ = self.list_.to_dict()

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "list": list_,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()

        def _parse_list_(
            data: object,
        ) -> Union[
            None,
            PythonToJsonschemaResponse200ArgsItemTypType4ListType0,
            PythonToJsonschemaResponse200ArgsItemTypType4ListType1,
        ]:
            if data is None:
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                list_type_0 = PythonToJsonschemaResponse200ArgsItemTypType4ListType0(data)

                return list_type_0
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            list_type_1 = PythonToJsonschemaResponse200ArgsItemTypType4ListType1.from_dict(data)

            return list_type_1

        list_ = _parse_list_(d.pop("list"))

        python_to_jsonschema_response_200_args_item_typ_type_4 = cls(
            list_=list_,
        )

        python_to_jsonschema_response_200_args_item_typ_type_4.additional_properties = d
        return python_to_jsonschema_response_200_args_item_typ_type_4

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
