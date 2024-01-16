from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="PolarsConnectionSettingsV2Response200StorageOptions")


@_attrs_define
class PolarsConnectionSettingsV2Response200StorageOptions:
    """ 
        Attributes:
            aws_endpoint_url (str):
            aws_region (str):
            aws_allow_http (str):
            aws_access_key_id (Union[Unset, str]):
            aws_secret_access_key (Union[Unset, str]):
     """

    aws_endpoint_url: str
    aws_region: str
    aws_allow_http: str
    aws_access_key_id: Union[Unset, str] = UNSET
    aws_secret_access_key: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        aws_endpoint_url = self.aws_endpoint_url
        aws_region = self.aws_region
        aws_allow_http = self.aws_allow_http
        aws_access_key_id = self.aws_access_key_id
        aws_secret_access_key = self.aws_secret_access_key

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "aws_endpoint_url": aws_endpoint_url,
            "aws_region": aws_region,
            "aws_allow_http": aws_allow_http,
        })
        if aws_access_key_id is not UNSET:
            field_dict["aws_access_key_id"] = aws_access_key_id
        if aws_secret_access_key is not UNSET:
            field_dict["aws_secret_access_key"] = aws_secret_access_key

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        aws_endpoint_url = d.pop("aws_endpoint_url")

        aws_region = d.pop("aws_region")

        aws_allow_http = d.pop("aws_allow_http")

        aws_access_key_id = d.pop("aws_access_key_id", UNSET)

        aws_secret_access_key = d.pop("aws_secret_access_key", UNSET)

        polars_connection_settings_v2_response_200_storage_options = cls(
            aws_endpoint_url=aws_endpoint_url,
            aws_region=aws_region,
            aws_allow_http=aws_allow_http,
            aws_access_key_id=aws_access_key_id,
            aws_secret_access_key=aws_secret_access_key,
        )

        polars_connection_settings_v2_response_200_storage_options.additional_properties = d
        return polars_connection_settings_v2_response_200_storage_options

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
