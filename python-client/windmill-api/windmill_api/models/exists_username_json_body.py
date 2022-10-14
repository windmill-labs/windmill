from typing import Any, Dict, List, Type, TypeVar

import attr

T = TypeVar("T", bound="ExistsUsernameJsonBody")


@attr.s(auto_attribs=True)
class ExistsUsernameJsonBody:
    """
    Attributes:
        id (str):
        username (str):
    """

    id: str
    username: str
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        id = self.id
        username = self.username

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "username": username,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        id = d.pop("id")

        username = d.pop("username")

        exists_username_json_body = cls(
            id=id,
            username=username,
        )

        exists_username_json_body.additional_properties = d
        return exists_username_json_body

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
