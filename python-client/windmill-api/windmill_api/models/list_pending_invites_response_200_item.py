from typing import Any, Dict, List, Type, TypeVar

import attr

T = TypeVar("T", bound="ListPendingInvitesResponse200Item")


@attr.s(auto_attribs=True)
class ListPendingInvitesResponse200Item:
    """
    Attributes:
        workspace_id (str):
        email (str):
        is_admin (bool):
    """

    workspace_id: str
    email: str
    is_admin: bool
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        workspace_id = self.workspace_id
        email = self.email
        is_admin = self.is_admin

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "workspace_id": workspace_id,
                "email": email,
                "is_admin": is_admin,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        workspace_id = d.pop("workspace_id")

        email = d.pop("email")

        is_admin = d.pop("is_admin")

        list_pending_invites_response_200_item = cls(
            workspace_id=workspace_id,
            email=email,
            is_admin=is_admin,
        )

        list_pending_invites_response_200_item.additional_properties = d
        return list_pending_invites_response_200_item

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
