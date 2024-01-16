from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast
from typing import Dict

if TYPE_CHECKING:
  from ..models.get_app_by_path_response_200_policy_triggerables_additional_property import GetAppByPathResponse200PolicyTriggerablesAdditionalProperty





T = TypeVar("T", bound="GetAppByPathResponse200PolicyTriggerables")


@_attrs_define
class GetAppByPathResponse200PolicyTriggerables:
    """ 
     """

    additional_properties: Dict[str, 'GetAppByPathResponse200PolicyTriggerablesAdditionalProperty'] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.get_app_by_path_response_200_policy_triggerables_additional_property import GetAppByPathResponse200PolicyTriggerablesAdditionalProperty
        
        field_dict: Dict[str, Any] = {}
        for prop_name, prop in self.additional_properties.items():
            field_dict[prop_name] = prop.to_dict()

        field_dict.update({
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.get_app_by_path_response_200_policy_triggerables_additional_property import GetAppByPathResponse200PolicyTriggerablesAdditionalProperty
        d = src_dict.copy()
        get_app_by_path_response_200_policy_triggerables = cls(
        )


        additional_properties = {}
        for prop_name, prop_dict in d.items():
            additional_property = GetAppByPathResponse200PolicyTriggerablesAdditionalProperty.from_dict(prop_dict)



            additional_properties[prop_name] = additional_property

        get_app_by_path_response_200_policy_triggerables.additional_properties = additional_properties
        return get_app_by_path_response_200_policy_triggerables

    @property
    def additional_keys(self) -> List[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> 'GetAppByPathResponse200PolicyTriggerablesAdditionalProperty':
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: 'GetAppByPathResponse200PolicyTriggerablesAdditionalProperty') -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
