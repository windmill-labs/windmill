from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast, List






T = TypeVar("T", bound="ListHubAppsResponse200AppsItem")


@_attrs_define
class ListHubAppsResponse200AppsItem:
    """ 
        Attributes:
            id (float):
            app_id (float):
            summary (str):
            apps (List[str]):
            approved (bool):
            votes (float):
     """

    id: float
    app_id: float
    summary: str
    apps: List[str]
    approved: bool
    votes: float
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        id = self.id
        app_id = self.app_id
        summary = self.summary
        apps = self.apps




        approved = self.approved
        votes = self.votes

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "id": id,
            "app_id": app_id,
            "summary": summary,
            "apps": apps,
            "approved": approved,
            "votes": votes,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        id = d.pop("id")

        app_id = d.pop("app_id")

        summary = d.pop("summary")

        apps = cast(List[str], d.pop("apps"))


        approved = d.pop("approved")

        votes = d.pop("votes")

        list_hub_apps_response_200_apps_item = cls(
            id=id,
            app_id=app_id,
            summary=summary,
            apps=apps,
            approved=approved,
            votes=votes,
        )

        list_hub_apps_response_200_apps_item.additional_properties = d
        return list_hub_apps_response_200_apps_item

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
