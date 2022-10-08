from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.get_hub_flow_by_id_response_200_flow_schema import GetHubFlowByIdResponse200FlowSchema
from ..models.get_hub_flow_by_id_response_200_flow_value import GetHubFlowByIdResponse200FlowValue
from ..types import UNSET, Unset

T = TypeVar("T", bound="GetHubFlowByIdResponse200Flow")


@attr.s(auto_attribs=True)
class GetHubFlowByIdResponse200Flow:
    """
    Attributes:
        summary (str):
        value (GetHubFlowByIdResponse200FlowValue):
        description (Union[Unset, str]):
        schema (Union[Unset, GetHubFlowByIdResponse200FlowSchema]):
    """

    summary: str
    value: GetHubFlowByIdResponse200FlowValue
    description: Union[Unset, str] = UNSET
    schema: Union[Unset, GetHubFlowByIdResponse200FlowSchema] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        summary = self.summary
        value = self.value.to_dict()

        description = self.description
        schema: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.schema, Unset):
            schema = self.schema.to_dict()

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "summary": summary,
                "value": value,
            }
        )
        if description is not UNSET:
            field_dict["description"] = description
        if schema is not UNSET:
            field_dict["schema"] = schema

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        summary = d.pop("summary")

        value = GetHubFlowByIdResponse200FlowValue.from_dict(d.pop("value"))

        description = d.pop("description", UNSET)

        _schema = d.pop("schema", UNSET)
        schema: Union[Unset, GetHubFlowByIdResponse200FlowSchema]
        if isinstance(_schema, Unset):
            schema = UNSET
        else:
            schema = GetHubFlowByIdResponse200FlowSchema.from_dict(_schema)

        get_hub_flow_by_id_response_200_flow = cls(
            summary=summary,
            value=value,
            description=description,
            schema=schema,
        )

        get_hub_flow_by_id_response_200_flow.additional_properties = d
        return get_hub_flow_by_id_response_200_flow

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
