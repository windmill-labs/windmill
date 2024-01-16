from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast
from typing import Dict

if TYPE_CHECKING:
  from ..models.polars_connection_settings_v2_response_200_storage_options import PolarsConnectionSettingsV2Response200StorageOptions
  from ..models.polars_connection_settings_v2_response_200s3_fs_args import PolarsConnectionSettingsV2Response200S3FsArgs





T = TypeVar("T", bound="PolarsConnectionSettingsV2Response200")


@_attrs_define
class PolarsConnectionSettingsV2Response200:
    """ 
        Attributes:
            s3fs_args (PolarsConnectionSettingsV2Response200S3FsArgs):
            storage_options (PolarsConnectionSettingsV2Response200StorageOptions):
     """

    s3fs_args: 'PolarsConnectionSettingsV2Response200S3FsArgs'
    storage_options: 'PolarsConnectionSettingsV2Response200StorageOptions'
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.polars_connection_settings_v2_response_200_storage_options import PolarsConnectionSettingsV2Response200StorageOptions
        from ..models.polars_connection_settings_v2_response_200s3_fs_args import PolarsConnectionSettingsV2Response200S3FsArgs
        s3fs_args = self.s3fs_args.to_dict()

        storage_options = self.storage_options.to_dict()


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "s3fs_args": s3fs_args,
            "storage_options": storage_options,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.polars_connection_settings_v2_response_200_storage_options import PolarsConnectionSettingsV2Response200StorageOptions
        from ..models.polars_connection_settings_v2_response_200s3_fs_args import PolarsConnectionSettingsV2Response200S3FsArgs
        d = src_dict.copy()
        s3fs_args = PolarsConnectionSettingsV2Response200S3FsArgs.from_dict(d.pop("s3fs_args"))




        storage_options = PolarsConnectionSettingsV2Response200StorageOptions.from_dict(d.pop("storage_options"))




        polars_connection_settings_v2_response_200 = cls(
            s3fs_args=s3fs_args,
            storage_options=storage_options,
        )

        polars_connection_settings_v2_response_200.additional_properties = d
        return polars_connection_settings_v2_response_200

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
