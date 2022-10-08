from typing import Any, Dict, List, Type, TypeVar

import attr

T = TypeVar("T", bound="AcceptInviteJsonBody")


@attr.s(auto_attribs=True)
class AcceptInviteJsonBody:
    """
    Attributes:
        workspace_id (str):
        username (str):
    """

    workspace_id: str
    username: str
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        workspace_id = self.workspace_id
        username = self.username

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "workspace_id": workspace_id,
                "username": username,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        workspace_id = d.pop("workspace_id")

        username = d.pop("username")

        accept_invite_json_body = cls(
            workspace_id=workspace_id,
            username=username,
        )

        accept_invite_json_body.additional_properties = d
        return accept_invite_json_body

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
