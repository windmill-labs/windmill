from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from typing import cast, List
from typing import Dict
from ..types import UNSET, Unset

if TYPE_CHECKING:
  from ..models.list_hub_flows_response_200_flows_item import ListHubFlowsResponse200FlowsItem





T = TypeVar("T", bound="ListHubFlowsResponse200")


@_attrs_define
class ListHubFlowsResponse200:
    """ 
        Attributes:
            flows (Union[Unset, List['ListHubFlowsResponse200FlowsItem']]):
     """

    flows: Union[Unset, List['ListHubFlowsResponse200FlowsItem']] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.list_hub_flows_response_200_flows_item import ListHubFlowsResponse200FlowsItem
        flows: Union[Unset, List[Dict[str, Any]]] = UNSET
        if not isinstance(self.flows, Unset):
            flows = []
            for flows_item_data in self.flows:
                flows_item = flows_item_data.to_dict()

                flows.append(flows_item)





        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if flows is not UNSET:
            field_dict["flows"] = flows

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.list_hub_flows_response_200_flows_item import ListHubFlowsResponse200FlowsItem
        d = src_dict.copy()
        flows = []
        _flows = d.pop("flows", UNSET)
        for flows_item_data in (_flows or []):
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
