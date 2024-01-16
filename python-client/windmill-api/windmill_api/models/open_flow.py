from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from ..types import UNSET, Unset
from typing import Dict

if TYPE_CHECKING:
  from ..models.open_flow_value import OpenFlowValue
  from ..models.open_flow_schema import OpenFlowSchema





T = TypeVar("T", bound="OpenFlow")


@_attrs_define
class OpenFlow:
    """ 
        Attributes:
            summary (str):
            value (OpenFlowValue):
            description (Union[Unset, str]):
            schema (Union[Unset, OpenFlowSchema]):
     """

    summary: str
    value: 'OpenFlowValue'
    description: Union[Unset, str] = UNSET
    schema: Union[Unset, 'OpenFlowSchema'] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.open_flow_value import OpenFlowValue
        from ..models.open_flow_schema import OpenFlowSchema
        summary = self.summary
        value = self.value.to_dict()

        description = self.description
        schema: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.schema, Unset):
            schema = self.schema.to_dict()


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "summary": summary,
            "value": value,
        })
        if description is not UNSET:
            field_dict["description"] = description
        if schema is not UNSET:
            field_dict["schema"] = schema

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.open_flow_value import OpenFlowValue
        from ..models.open_flow_schema import OpenFlowSchema
        d = src_dict.copy()
        summary = d.pop("summary")

        value = OpenFlowValue.from_dict(d.pop("value"))




        description = d.pop("description", UNSET)

        _schema = d.pop("schema", UNSET)
        schema: Union[Unset, OpenFlowSchema]
        if isinstance(_schema,  Unset):
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
