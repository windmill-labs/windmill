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
  from ..models.polars_connection_settings_response_200_client_kwargs import PolarsConnectionSettingsResponse200ClientKwargs





T = TypeVar("T", bound="PolarsConnectionSettingsResponse200")


@_attrs_define
class PolarsConnectionSettingsResponse200:
    """ 
        Attributes:
            endpoint_url (str):
            use_ssl (bool):
            cache_regions (bool):
            client_kwargs (PolarsConnectionSettingsResponse200ClientKwargs):
            key (Union[Unset, str]):
            secret (Union[Unset, str]):
     """

    endpoint_url: str
    use_ssl: bool
    cache_regions: bool
    client_kwargs: 'PolarsConnectionSettingsResponse200ClientKwargs'
    key: Union[Unset, str] = UNSET
    secret: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.polars_connection_settings_response_200_client_kwargs import PolarsConnectionSettingsResponse200ClientKwargs
        endpoint_url = self.endpoint_url
        use_ssl = self.use_ssl
        cache_regions = self.cache_regions
        client_kwargs = self.client_kwargs.to_dict()

        key = self.key
        secret = self.secret

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "endpoint_url": endpoint_url,
            "use_ssl": use_ssl,
            "cache_regions": cache_regions,
            "client_kwargs": client_kwargs,
        })
        if key is not UNSET:
            field_dict["key"] = key
        if secret is not UNSET:
            field_dict["secret"] = secret

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.polars_connection_settings_response_200_client_kwargs import PolarsConnectionSettingsResponse200ClientKwargs
        d = src_dict.copy()
        endpoint_url = d.pop("endpoint_url")

        use_ssl = d.pop("use_ssl")

        cache_regions = d.pop("cache_regions")

        client_kwargs = PolarsConnectionSettingsResponse200ClientKwargs.from_dict(d.pop("client_kwargs"))




        key = d.pop("key", UNSET)

        secret = d.pop("secret", UNSET)

        polars_connection_settings_response_200 = cls(
            endpoint_url=endpoint_url,
            use_ssl=use_ssl,
            cache_regions=cache_regions,
            client_kwargs=client_kwargs,
            key=key,
            secret=secret,
        )

        polars_connection_settings_response_200.additional_properties = d
        return polars_connection_settings_response_200

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
