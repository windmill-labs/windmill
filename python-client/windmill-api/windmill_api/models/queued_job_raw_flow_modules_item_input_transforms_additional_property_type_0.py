from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.queued_job_raw_flow_modules_item_input_transforms_additional_property_type_0_type import (
    QueuedJobRawFlowModulesItemInputTransformsAdditionalPropertyType0Type,
)
from ..types import UNSET, Unset

T = TypeVar("T", bound="QueuedJobRawFlowModulesItemInputTransformsAdditionalPropertyType0")


@attr.s(auto_attribs=True)
class QueuedJobRawFlowModulesItemInputTransformsAdditionalPropertyType0:
    """
    Attributes:
        type (QueuedJobRawFlowModulesItemInputTransformsAdditionalPropertyType0Type):
        value (Union[Unset, Any]):
    """

    type: QueuedJobRawFlowModulesItemInputTransformsAdditionalPropertyType0Type
    value: Union[Unset, Any] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        type = self.type.value

        value = self.value

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "type": type,
            }
        )
        if value is not UNSET:
            field_dict["value"] = value

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        type = QueuedJobRawFlowModulesItemInputTransformsAdditionalPropertyType0Type(d.pop("type"))

        value = d.pop("value", UNSET)

        queued_job_raw_flow_modules_item_input_transforms_additional_property_type_0 = cls(
            type=type,
            value=value,
        )

        queued_job_raw_flow_modules_item_input_transforms_additional_property_type_0.additional_properties = d
        return queued_job_raw_flow_modules_item_input_transforms_additional_property_type_0

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
