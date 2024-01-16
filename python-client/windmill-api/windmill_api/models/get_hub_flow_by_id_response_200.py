from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from ..types import UNSET, Unset
from typing import Dict

if TYPE_CHECKING:
  from ..models.get_hub_flow_by_id_response_200_flow import GetHubFlowByIdResponse200Flow





T = TypeVar("T", bound="GetHubFlowByIdResponse200")


@_attrs_define
class GetHubFlowByIdResponse200:
    """ 
        Attributes:
            flow (Union[Unset, GetHubFlowByIdResponse200Flow]):
     """

    flow: Union[Unset, 'GetHubFlowByIdResponse200Flow'] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.get_hub_flow_by_id_response_200_flow import GetHubFlowByIdResponse200Flow
        flow: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.flow, Unset):
            flow = self.flow.to_dict()


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if flow is not UNSET:
            field_dict["flow"] = flow

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.get_hub_flow_by_id_response_200_flow import GetHubFlowByIdResponse200Flow
        d = src_dict.copy()
        _flow = d.pop("flow", UNSET)
        flow: Union[Unset, GetHubFlowByIdResponse200Flow]
        if isinstance(_flow,  Unset):
            flow = UNSET
        else:
            flow = GetHubFlowByIdResponse200Flow.from_dict(_flow)




        get_hub_flow_by_id_response_200 = cls(
            flow=flow,
        )

        get_hub_flow_by_id_response_200.additional_properties = d
        return get_hub_flow_by_id_response_200

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
