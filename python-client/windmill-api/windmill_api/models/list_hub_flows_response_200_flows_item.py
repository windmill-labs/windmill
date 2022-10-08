from typing import Any, Dict, List, Type, TypeVar, cast

import attr

T = TypeVar("T", bound="ListHubFlowsResponse200FlowsItem")


@attr.s(auto_attribs=True)
class ListHubFlowsResponse200FlowsItem:
    """
    Attributes:
        id (float):
        flow_id (float):
        summary (str):
        apps (List[str]):
        approved (bool):
        votes (float):
    """

    id: float
    flow_id: float
    summary: str
    apps: List[str]
    approved: bool
    votes: float
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        id = self.id
        flow_id = self.flow_id
        summary = self.summary
        apps = self.apps

        approved = self.approved
        votes = self.votes

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "flow_id": flow_id,
                "summary": summary,
                "apps": apps,
                "approved": approved,
                "votes": votes,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        id = d.pop("id")

        flow_id = d.pop("flow_id")

        summary = d.pop("summary")

        apps = cast(List[str], d.pop("apps"))

        approved = d.pop("approved")

        votes = d.pop("votes")

        list_hub_flows_response_200_flows_item = cls(
            id=id,
            flow_id=flow_id,
            summary=summary,
            apps=apps,
            approved=approved,
            votes=votes,
        )

        list_hub_flows_response_200_flows_item.additional_properties = d
        return list_hub_flows_response_200_flows_item

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
