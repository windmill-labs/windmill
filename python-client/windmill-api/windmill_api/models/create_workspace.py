from typing import Any, Dict, List, Type, TypeVar

import attr

T = TypeVar("T", bound="CreateWorkspace")


@attr.s(auto_attribs=True)
class CreateWorkspace:
    """
    Attributes:
        id (str):
        name (str):
        username (str):
        domain (str):
    """

    id: str
    name: str
    username: str
    domain: str
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        id = self.id
        name = self.name
        username = self.username
        domain = self.domain

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "name": name,
                "username": username,
                "domain": domain,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        id = d.pop("id")

        name = d.pop("name")

        username = d.pop("username")

        domain = d.pop("domain")

        create_workspace = cls(
            id=id,
            name=name,
            username=username,
            domain=domain,
        )

        create_workspace.additional_properties = d
        return create_workspace

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
