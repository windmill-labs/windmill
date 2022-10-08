from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.get_hub_flow_by_id_response_200_flow_value_failure_module_input_transforms_additional_property_type_0 import (
    GetHubFlowByIdResponse200FlowValueFailureModuleInputTransformsAdditionalPropertyType0,
)
from ..models.get_hub_flow_by_id_response_200_flow_value_failure_module_input_transforms_additional_property_type_1 import (
    GetHubFlowByIdResponse200FlowValueFailureModuleInputTransformsAdditionalPropertyType1,
)

T = TypeVar("T", bound="GetHubFlowByIdResponse200FlowValueFailureModuleInputTransforms")


@attr.s(auto_attribs=True)
class GetHubFlowByIdResponse200FlowValueFailureModuleInputTransforms:
    """ """

    additional_properties: Dict[
        str,
        Union[
            GetHubFlowByIdResponse200FlowValueFailureModuleInputTransformsAdditionalPropertyType0,
            GetHubFlowByIdResponse200FlowValueFailureModuleInputTransformsAdditionalPropertyType1,
        ],
    ] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:

        field_dict: Dict[str, Any] = {}
        for prop_name, prop in self.additional_properties.items():

            if isinstance(prop, GetHubFlowByIdResponse200FlowValueFailureModuleInputTransformsAdditionalPropertyType0):
                field_dict[prop_name] = prop.to_dict()

            else:
                field_dict[prop_name] = prop.to_dict()

        field_dict.update({})

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        get_hub_flow_by_id_response_200_flow_value_failure_module_input_transforms = cls()

        additional_properties = {}
        for prop_name, prop_dict in d.items():

            def _parse_additional_property(
                data: object,
            ) -> Union[
                GetHubFlowByIdResponse200FlowValueFailureModuleInputTransformsAdditionalPropertyType0,
                GetHubFlowByIdResponse200FlowValueFailureModuleInputTransformsAdditionalPropertyType1,
            ]:
                try:
                    if not isinstance(data, dict):
                        raise TypeError()
                    additional_property_type_0 = (
                        GetHubFlowByIdResponse200FlowValueFailureModuleInputTransformsAdditionalPropertyType0.from_dict(
                            data
                        )
                    )

                    return additional_property_type_0
                except:  # noqa: E722
                    pass
                if not isinstance(data, dict):
                    raise TypeError()
                additional_property_type_1 = (
                    GetHubFlowByIdResponse200FlowValueFailureModuleInputTransformsAdditionalPropertyType1.from_dict(
                        data
                    )
                )

                return additional_property_type_1

            additional_property = _parse_additional_property(prop_dict)

            additional_properties[prop_name] = additional_property

        get_hub_flow_by_id_response_200_flow_value_failure_module_input_transforms.additional_properties = (
            additional_properties
        )
        return get_hub_flow_by_id_response_200_flow_value_failure_module_input_transforms

    @property
    def additional_keys(self) -> List[str]:
        return list(self.additional_properties.keys())

    def __getitem__(
        self, key: str
    ) -> Union[
        GetHubFlowByIdResponse200FlowValueFailureModuleInputTransformsAdditionalPropertyType0,
        GetHubFlowByIdResponse200FlowValueFailureModuleInputTransformsAdditionalPropertyType1,
    ]:
        return self.additional_properties[key]

    def __setitem__(
        self,
        key: str,
        value: Union[
            GetHubFlowByIdResponse200FlowValueFailureModuleInputTransformsAdditionalPropertyType0,
            GetHubFlowByIdResponse200FlowValueFailureModuleInputTransformsAdditionalPropertyType1,
        ],
    ) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
