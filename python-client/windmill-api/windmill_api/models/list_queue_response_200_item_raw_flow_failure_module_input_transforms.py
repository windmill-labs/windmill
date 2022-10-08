from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.list_queue_response_200_item_raw_flow_failure_module_input_transforms_additional_property_type_0 import (
    ListQueueResponse200ItemRawFlowFailureModuleInputTransformsAdditionalPropertyType0,
)
from ..models.list_queue_response_200_item_raw_flow_failure_module_input_transforms_additional_property_type_1 import (
    ListQueueResponse200ItemRawFlowFailureModuleInputTransformsAdditionalPropertyType1,
)

T = TypeVar("T", bound="ListQueueResponse200ItemRawFlowFailureModuleInputTransforms")


@attr.s(auto_attribs=True)
class ListQueueResponse200ItemRawFlowFailureModuleInputTransforms:
    """ """

    additional_properties: Dict[
        str,
        Union[
            ListQueueResponse200ItemRawFlowFailureModuleInputTransformsAdditionalPropertyType0,
            ListQueueResponse200ItemRawFlowFailureModuleInputTransformsAdditionalPropertyType1,
        ],
    ] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:

        field_dict: Dict[str, Any] = {}
        for prop_name, prop in self.additional_properties.items():

            if isinstance(prop, ListQueueResponse200ItemRawFlowFailureModuleInputTransformsAdditionalPropertyType0):
                field_dict[prop_name] = prop.to_dict()

            else:
                field_dict[prop_name] = prop.to_dict()

        field_dict.update({})

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        list_queue_response_200_item_raw_flow_failure_module_input_transforms = cls()

        additional_properties = {}
        for prop_name, prop_dict in d.items():

            def _parse_additional_property(
                data: object,
            ) -> Union[
                ListQueueResponse200ItemRawFlowFailureModuleInputTransformsAdditionalPropertyType0,
                ListQueueResponse200ItemRawFlowFailureModuleInputTransformsAdditionalPropertyType1,
            ]:
                try:
                    if not isinstance(data, dict):
                        raise TypeError()
                    additional_property_type_0 = (
                        ListQueueResponse200ItemRawFlowFailureModuleInputTransformsAdditionalPropertyType0.from_dict(
                            data
                        )
                    )

                    return additional_property_type_0
                except:  # noqa: E722
                    pass
                if not isinstance(data, dict):
                    raise TypeError()
                additional_property_type_1 = (
                    ListQueueResponse200ItemRawFlowFailureModuleInputTransformsAdditionalPropertyType1.from_dict(data)
                )

                return additional_property_type_1

            additional_property = _parse_additional_property(prop_dict)

            additional_properties[prop_name] = additional_property

        list_queue_response_200_item_raw_flow_failure_module_input_transforms.additional_properties = (
            additional_properties
        )
        return list_queue_response_200_item_raw_flow_failure_module_input_transforms

    @property
    def additional_keys(self) -> List[str]:
        return list(self.additional_properties.keys())

    def __getitem__(
        self, key: str
    ) -> Union[
        ListQueueResponse200ItemRawFlowFailureModuleInputTransformsAdditionalPropertyType0,
        ListQueueResponse200ItemRawFlowFailureModuleInputTransformsAdditionalPropertyType1,
    ]:
        return self.additional_properties[key]

    def __setitem__(
        self,
        key: str,
        value: Union[
            ListQueueResponse200ItemRawFlowFailureModuleInputTransformsAdditionalPropertyType0,
            ListQueueResponse200ItemRawFlowFailureModuleInputTransformsAdditionalPropertyType1,
        ],
    ) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
