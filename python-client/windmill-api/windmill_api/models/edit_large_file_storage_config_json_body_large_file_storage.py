from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset
from ..models.edit_large_file_storage_config_json_body_large_file_storage_type import EditLargeFileStorageConfigJsonBodyLargeFileStorageType






T = TypeVar("T", bound="EditLargeFileStorageConfigJsonBodyLargeFileStorage")


@_attrs_define
class EditLargeFileStorageConfigJsonBodyLargeFileStorage:
    """ 
        Attributes:
            type (Union[Unset, EditLargeFileStorageConfigJsonBodyLargeFileStorageType]):
            s3_resource_path (Union[Unset, str]):
     """

    type: Union[Unset, EditLargeFileStorageConfigJsonBodyLargeFileStorageType] = UNSET
    s3_resource_path: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        type: Union[Unset, str] = UNSET
        if not isinstance(self.type, Unset):
            type = self.type.value

        s3_resource_path = self.s3_resource_path

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if type is not UNSET:
            field_dict["type"] = type
        if s3_resource_path is not UNSET:
            field_dict["s3_resource_path"] = s3_resource_path

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        _type = d.pop("type", UNSET)
        type: Union[Unset, EditLargeFileStorageConfigJsonBodyLargeFileStorageType]
        if isinstance(_type,  Unset):
            type = UNSET
        else:
            type = EditLargeFileStorageConfigJsonBodyLargeFileStorageType(_type)




        s3_resource_path = d.pop("s3_resource_path", UNSET)

        edit_large_file_storage_config_json_body_large_file_storage = cls(
            type=type,
            s3_resource_path=s3_resource_path,
        )

        edit_large_file_storage_config_json_body_large_file_storage.additional_properties = d
        return edit_large_file_storage_config_json_body_large_file_storage

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
