from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.list_hub_flows_response_200_flows_item import ListHubFlowsResponse200FlowsItem
from ..types import UNSET, Unset

T = TypeVar("T", bound="ListHubFlowsResponse200")


@attr.s(auto_attribs=True)
class ListHubFlowsResponse200:
    """
    Attributes:
        flows (Union[Unset, List[ListHubFlowsResponse200FlowsItem]]):
    """

    flows: Union[Unset, List[ListHubFlowsResponse200FlowsItem]] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        flows: Union[Unset, List[Dict[str, Any]]] = UNSET
        if not isinstance(self.flows, Unset):
            flows = []
            for flows_item_data in self.flows:
                flows_item = flows_item_data.to_dict()

                flows.append(flows_item)

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if flows is not UNSET:
            field_dict["flows"] = flows

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        flows = []
        _flows = d.pop("flows", UNSET)
        for flows_item_data in _flows or []:
            flows_item = ListHubFlowsResponse200FlowsItem.from_dict(flows_item_data)

            flows.append(flows_item)

        list_hub_flows_response_200 = cls(
            flows=flows,
        )

        list_hub_flows_response_200.additional_properties = d
        return list_hub_flows_response_200

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
