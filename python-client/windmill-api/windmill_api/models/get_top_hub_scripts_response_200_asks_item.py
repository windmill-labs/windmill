from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset







T = TypeVar("T", bound="GetTopHubScriptsResponse200AsksItem")


@_attrs_define
class GetTopHubScriptsResponse200AsksItem:
    """ 
        Attributes:
            id (float):
            ask_id (float):
            summary (str):
            app (str):
            version_id (float):
            kind (Any):
            votes (float):
            views (float):
     """

    id: float
    ask_id: float
    summary: str
    app: str
    version_id: float
    kind: Any
    votes: float
    views: float
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        id = self.id
        ask_id = self.ask_id
        summary = self.summary
        app = self.app
        version_id = self.version_id
        kind = self.kind
        votes = self.votes
        views = self.views

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "id": id,
            "ask_id": ask_id,
            "summary": summary,
            "app": app,
            "version_id": version_id,
            "kind": kind,
            "votes": votes,
            "views": views,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        id = d.pop("id")

        ask_id = d.pop("ask_id")

        summary = d.pop("summary")

        app = d.pop("app")

        version_id = d.pop("version_id")

        kind = d.pop("kind")

        votes = d.pop("votes")

        views = d.pop("views")

        get_top_hub_scripts_response_200_asks_item = cls(
            id=id,
            ask_id=ask_id,
            summary=summary,
            app=app,
            version_id=version_id,
            kind=kind,
            votes=votes,
            views=views,
        )

        get_top_hub_scripts_response_200_asks_item.additional_properties = d
        return get_top_hub_scripts_response_200_asks_item

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
