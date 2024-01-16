from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast
from typing import Dict

if TYPE_CHECKING:
  from ..models.policy_triggerables_additional_property import PolicyTriggerablesAdditionalProperty





T = TypeVar("T", bound="PolicyTriggerables")


@_attrs_define
class PolicyTriggerables:
    """ 
     """

    additional_properties: Dict[str, 'PolicyTriggerablesAdditionalProperty'] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.policy_triggerables_additional_property import PolicyTriggerablesAdditionalProperty
        
        field_dict: Dict[str, Any] = {}
        for prop_name, prop in self.additional_properties.items():
            field_dict[prop_name] = prop.to_dict()

        field_dict.update({
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.policy_triggerables_additional_property import PolicyTriggerablesAdditionalProperty
        d = src_dict.copy()
        policy_triggerables = cls(
        )


        additional_properties = {}
        for prop_name, prop_dict in d.items():
            additional_property = PolicyTriggerablesAdditionalProperty.from_dict(prop_dict)



            additional_properties[prop_name] = additional_property

        policy_triggerables.additional_properties = additional_properties
        return policy_triggerables

    @property
    def additional_keys(self) -> List[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> 'PolicyTriggerablesAdditionalProperty':
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: 'PolicyTriggerablesAdditionalProperty') -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
