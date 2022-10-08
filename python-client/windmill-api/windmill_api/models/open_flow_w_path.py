from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.open_flow_w_path_schema import OpenFlowWPathSchema
from ..models.open_flow_w_path_value import OpenFlowWPathValue
from ..types import UNSET, Unset

T = TypeVar("T", bound="OpenFlowWPath")


@attr.s(auto_attribs=True)
class OpenFlowWPath:
    """
    Attributes:
        summary (str):
        value (OpenFlowWPathValue):
        path (str):
        description (Union[Unset, str]):
        schema (Union[Unset, OpenFlowWPathSchema]):
    """

    summary: str
    value: OpenFlowWPathValue
    path: str
    description: Union[Unset, str] = UNSET
    schema: Union[Unset, OpenFlowWPathSchema] = UNSET
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

        value = OpenFlowWPathValue.from_dict(d.pop("value"))

        path = d.pop("path")

        description = d.pop("description", UNSET)

        _schema = d.pop("schema", UNSET)
        schema: Union[Unset, OpenFlowWPathSchema]
        if isinstance(_schema, Unset):
            schema = UNSET
        else:
            schema = OpenFlowWPathSchema.from_dict(_schema)

        open_flow_w_path = cls(
            summary=summary,
            value=value,
            path=path,
            description=description,
            schema=schema,
        )

        open_flow_w_path.additional_properties = d
        return open_flow_w_path

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
