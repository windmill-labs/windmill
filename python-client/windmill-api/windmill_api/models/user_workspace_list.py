from typing import Any, Dict, List, Type, TypeVar

import attr

from ..models.user_workspace_list_workspaces_item import UserWorkspaceListWorkspacesItem

T = TypeVar("T", bound="UserWorkspaceList")


@attr.s(auto_attribs=True)
class UserWorkspaceList:
    """
    Attributes:
        email (str):
        workspaces (List[UserWorkspaceListWorkspacesItem]):
    """

    email: str
    workspaces: List[UserWorkspaceListWorkspacesItem]
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        email = self.email
        workspaces = []
        for workspaces_item_data in self.workspaces:
            workspaces_item = workspaces_item_data.to_dict()

            workspaces.append(workspaces_item)

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "email": email,
                "workspaces": workspaces,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        email = d.pop("email")

        workspaces = []
        _workspaces = d.pop("workspaces")
        for workspaces_item_data in _workspaces:
            workspaces_item = UserWorkspaceListWorkspacesItem.from_dict(workspaces_item_data)

            workspaces.append(workspaces_item)

        user_workspace_list = cls(
            email=email,
            workspaces=workspaces,
        )

        user_workspace_list.additional_properties = d
        return user_workspace_list

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
