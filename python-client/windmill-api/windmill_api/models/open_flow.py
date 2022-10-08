from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.open_flow_schema import OpenFlowSchema
from ..models.open_flow_value import OpenFlowValue
from ..types import UNSET, Unset

T = TypeVar("T", bound="OpenFlow")


@attr.s(auto_attribs=True)
class OpenFlow:
    """
    Attributes:
        summary (str):
        value (OpenFlowValue):
        description (Union[Unset, str]):
        schema (Union[Unset, OpenFlowSchema]):
    """

    summary: str
    value: OpenFlowValue
    description: Union[Unset, str] = UNSET
    schema: Union[Unset, OpenFlowSchema] = UNSET
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

        value = OpenFlowValue.from_dict(d.pop("value"))

        description = d.pop("description", UNSET)

        _schema = d.pop("schema", UNSET)
        schema: Union[Unset, OpenFlowSchema]
        if isinstance(_schema, Unset):
            schema = UNSET
        else:
            schema = OpenFlowSchema.from_dict(_schema)

        open_flow = cls(
            summary=summary,
            value=value,
            description=description,
            schema=schema,
        )

        open_flow.additional_properties = d
        return open_flow

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
