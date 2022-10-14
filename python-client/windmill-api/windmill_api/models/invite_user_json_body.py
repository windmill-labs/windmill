from typing import Any, Dict, List, Type, TypeVar

import attr

T = TypeVar("T", bound="InviteUserJsonBody")


@attr.s(auto_attribs=True)
class InviteUserJsonBody:
    """
    Attributes:
        email (str):
        is_admin (bool):
    """

    email: str
    is_admin: bool
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        email = self.email
        is_admin = self.is_admin

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "email": email,
                "is_admin": is_admin,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        email = d.pop("email")

        is_admin = d.pop("is_admin")

        invite_user_json_body = cls(
            email=email,
            is_admin=is_admin,
        )

        invite_user_json_body.additional_properties = d
        return invite_user_json_body

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
