from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset







T = TypeVar("T", bound="DuckdbConnectionSettingsV2Response200")


@_attrs_define
class DuckdbConnectionSettingsV2Response200:
    """ 
        Attributes:
            connection_settings_str (str):
     """

    connection_settings_str: str
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        connection_settings_str = self.connection_settings_str

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "connection_settings_str": connection_settings_str,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        connection_settings_str = d.pop("connection_settings_str")

        duckdb_connection_settings_v2_response_200 = cls(
            connection_settings_str=connection_settings_str,
        )

        duckdb_connection_settings_v2_response_200.additional_properties = d
        return duckdb_connection_settings_v2_response_200

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
