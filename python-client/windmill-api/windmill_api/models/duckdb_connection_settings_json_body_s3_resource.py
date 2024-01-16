from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="DuckdbConnectionSettingsJsonBodyS3Resource")


@_attrs_define
class DuckdbConnectionSettingsJsonBodyS3Resource:
    """ 
        Attributes:
            bucket (str):
            region (str):
            end_point (str):
            use_ssl (bool):
            path_style (bool):
            access_key (Union[Unset, str]):
            secret_key (Union[Unset, str]):
     """

    bucket: str
    region: str
    end_point: str
    use_ssl: bool
    path_style: bool
    access_key: Union[Unset, str] = UNSET
    secret_key: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        bucket = self.bucket
        region = self.region
        end_point = self.end_point
        use_ssl = self.use_ssl
        path_style = self.path_style
        access_key = self.access_key
        secret_key = self.secret_key

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "bucket": bucket,
            "region": region,
            "endPoint": end_point,
            "useSSL": use_ssl,
            "pathStyle": path_style,
        })
        if access_key is not UNSET:
            field_dict["accessKey"] = access_key
        if secret_key is not UNSET:
            field_dict["secretKey"] = secret_key

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        bucket = d.pop("bucket")

        region = d.pop("region")

        end_point = d.pop("endPoint")

        use_ssl = d.pop("useSSL")

        path_style = d.pop("pathStyle")

        access_key = d.pop("accessKey", UNSET)

        secret_key = d.pop("secretKey", UNSET)

        duckdb_connection_settings_json_body_s3_resource = cls(
            bucket=bucket,
            region=region,
            end_point=end_point,
            use_ssl=use_ssl,
            path_style=path_style,
            access_key=access_key,
            secret_key=secret_key,
        )

        duckdb_connection_settings_json_body_s3_resource.additional_properties = d
        return duckdb_connection_settings_json_body_s3_resource

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
