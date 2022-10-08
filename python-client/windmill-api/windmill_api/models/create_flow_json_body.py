from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.create_flow_json_body_schema import CreateFlowJsonBodySchema
from ..models.create_flow_json_body_value import CreateFlowJsonBodyValue
from ..types import UNSET, Unset

T = TypeVar("T", bound="CreateFlowJsonBody")


@attr.s(auto_attribs=True)
class CreateFlowJsonBody:
    """
    Attributes:
        summary (str):
        value (CreateFlowJsonBodyValue):
        path (str):
        description (Union[Unset, str]):
        schema (Union[Unset, CreateFlowJsonBodySchema]):
    """

    summary: str
    value: CreateFlowJsonBodyValue
    path: str
    description: Union[Unset, str] = UNSET
    schema: Union[Unset, CreateFlowJsonBodySchema] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        summary = self.summary
        value = self.value.to_dict()

        path = self.path
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
                "path": path,
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

        value = CreateFlowJsonBodyValue.from_dict(d.pop("value"))

        path = d.pop("path")

        description = d.pop("description", UNSET)

        _schema = d.pop("schema", UNSET)
        schema: Union[Unset, CreateFlowJsonBodySchema]
        if isinstance(_schema, Unset):
            schema = UNSET
        else:
            schema = CreateFlowJsonBodySchema.from_dict(_schema)

        create_flow_json_body = cls(
            summary=summary,
            value=value,
            path=path,
            description=description,
            schema=schema,
        )

        create_flow_json_body.additional_properties = d
        return create_flow_json_body

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
