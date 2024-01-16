from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast
from typing import Dict
from typing import cast, List

if TYPE_CHECKING:
  from ..models.list_user_workspaces_response_200_workspaces_item import ListUserWorkspacesResponse200WorkspacesItem





T = TypeVar("T", bound="ListUserWorkspacesResponse200")


@_attrs_define
class ListUserWorkspacesResponse200:
    """ 
        Attributes:
            email (str):
            workspaces (List['ListUserWorkspacesResponse200WorkspacesItem']):
     """

    email: str
    workspaces: List['ListUserWorkspacesResponse200WorkspacesItem']
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.list_user_workspaces_response_200_workspaces_item import ListUserWorkspacesResponse200WorkspacesItem
        email = self.email
        workspaces = []
        for workspaces_item_data in self.workspaces:
            workspaces_item = workspaces_item_data.to_dict()

            workspaces.append(workspaces_item)





        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "email": email,
            "workspaces": workspaces,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.list_user_workspaces_response_200_workspaces_item import ListUserWorkspacesResponse200WorkspacesItem
        d = src_dict.copy()
        email = d.pop("email")

        workspaces = []
        _workspaces = d.pop("workspaces")
        for workspaces_item_data in (_workspaces):
            workspaces_item = ListUserWorkspacesResponse200WorkspacesItem.from_dict(workspaces_item_data)



            workspaces.append(workspaces_item)


        list_user_workspaces_response_200 = cls(
            email=email,
            workspaces=workspaces,
        )

        list_user_workspaces_response_200.additional_properties = d
        return list_user_workspaces_response_200

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
