from typing import Any, Dict, List, Type, TypeVar

import attr

T = TypeVar("T", bound="ListContextualVariablesResponse200Item")


@attr.s(auto_attribs=True)
class ListContextualVariablesResponse200Item:
    """
    Attributes:
        name (str):
        value (str):
        description (str):
    """

    name: str
    value: str
    description: str
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        name = self.name
        value = self.value
        description = self.description

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "name": name,
                "value": value,
                "description": description,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        name = d.pop("name")

        value = d.pop("value")

        description = d.pop("description")

        list_contextual_variables_response_200_item = cls(
            name=name,
            value=value,
            description=description,
        )

        list_contextual_variables_response_200_item.additional_properties = d
        return list_contextual_variables_response_200_item

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
