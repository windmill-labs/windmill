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
  from ..models.duckdb_connection_settings_json_body_s3_resource import DuckdbConnectionSettingsJsonBodyS3Resource





T = TypeVar("T", bound="DuckdbConnectionSettingsJsonBody")


@_attrs_define
class DuckdbConnectionSettingsJsonBody:
    """ 
        Attributes:
            s3_resource (Union[Unset, DuckdbConnectionSettingsJsonBodyS3Resource]):
     """

    s3_resource: Union[Unset, 'DuckdbConnectionSettingsJsonBodyS3Resource'] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.duckdb_connection_settings_json_body_s3_resource import DuckdbConnectionSettingsJsonBodyS3Resource
        s3_resource: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.s3_resource, Unset):
            s3_resource = self.s3_resource.to_dict()


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if s3_resource is not UNSET:
            field_dict["s3_resource"] = s3_resource

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.duckdb_connection_settings_json_body_s3_resource import DuckdbConnectionSettingsJsonBodyS3Resource
        d = src_dict.copy()
        _s3_resource = d.pop("s3_resource", UNSET)
        s3_resource: Union[Unset, DuckdbConnectionSettingsJsonBodyS3Resource]
        if isinstance(_s3_resource,  Unset):
            s3_resource = UNSET
        else:
            s3_resource = DuckdbConnectionSettingsJsonBodyS3Resource.from_dict(_s3_resource)




        duckdb_connection_settings_json_body = cls(
            s3_resource=s3_resource,
        )

        duckdb_connection_settings_json_body.additional_properties = d
        return duckdb_connection_settings_json_body

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
