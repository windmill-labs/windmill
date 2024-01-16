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
  from ..models.edit_large_file_storage_config_json_body_large_file_storage import EditLargeFileStorageConfigJsonBodyLargeFileStorage





T = TypeVar("T", bound="EditLargeFileStorageConfigJsonBody")


@_attrs_define
class EditLargeFileStorageConfigJsonBody:
    """ 
        Attributes:
            large_file_storage (Union[Unset, EditLargeFileStorageConfigJsonBodyLargeFileStorage]):
     """

    large_file_storage: Union[Unset, 'EditLargeFileStorageConfigJsonBodyLargeFileStorage'] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.edit_large_file_storage_config_json_body_large_file_storage import EditLargeFileStorageConfigJsonBodyLargeFileStorage
        large_file_storage: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.large_file_storage, Unset):
            large_file_storage = self.large_file_storage.to_dict()


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if large_file_storage is not UNSET:
            field_dict["large_file_storage"] = large_file_storage

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.edit_large_file_storage_config_json_body_large_file_storage import EditLargeFileStorageConfigJsonBodyLargeFileStorage
        d = src_dict.copy()
        _large_file_storage = d.pop("large_file_storage", UNSET)
        large_file_storage: Union[Unset, EditLargeFileStorageConfigJsonBodyLargeFileStorage]
        if isinstance(_large_file_storage,  Unset):
            large_file_storage = UNSET
        else:
            large_file_storage = EditLargeFileStorageConfigJsonBodyLargeFileStorage.from_dict(_large_file_storage)




        edit_large_file_storage_config_json_body = cls(
            large_file_storage=large_file_storage,
        )

        edit_large_file_storage_config_json_body.additional_properties = d
        return edit_large_file_storage_config_json_body

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
