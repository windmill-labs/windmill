from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..types import UNSET, Unset

T = TypeVar("T", bound="CreateResourceTypeJsonBody")


@attr.s(auto_attribs=True)
class CreateResourceTypeJsonBody:
    """
    Attributes:
        name (str):
        workspace_id (Union[Unset, str]):
        schema (Union[Unset, Any]):
        description (Union[Unset, str]):
    """

    name: str
    workspace_id: Union[Unset, str] = UNSET
    schema: Union[Unset, Any] = UNSET
    description: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        name = self.name
        workspace_id = self.workspace_id
        schema = self.schema
        description = self.description

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "name": name,
            }
        )
        if workspace_id is not UNSET:
            field_dict["workspace_id"] = workspace_id
        if schema is not UNSET:
            field_dict["schema"] = schema
        if description is not UNSET:
            field_dict["description"] = description

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        name = d.pop("name")

        workspace_id = d.pop("workspace_id", UNSET)

        schema = d.pop("schema", UNSET)

        description = d.pop("description", UNSET)

        create_resource_type_json_body = cls(
            name=name,
            workspace_id=workspace_id,
            schema=schema,
            description=description,
        )

        create_resource_type_json_body.additional_properties = d
        return create_resource_type_json_body

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
