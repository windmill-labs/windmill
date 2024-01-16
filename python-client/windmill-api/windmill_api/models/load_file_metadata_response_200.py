from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
import datetime
from dateutil.parser import isoparse
from ..types import UNSET, Unset






T = TypeVar("T", bound="LoadFileMetadataResponse200")


@_attrs_define
class LoadFileMetadataResponse200:
    """ 
        Attributes:
            mime_type (Union[Unset, str]):
            size_in_bytes (Union[Unset, int]):
            last_modified (Union[Unset, datetime.datetime]):
            expires (Union[Unset, datetime.datetime]):
            version_id (Union[Unset, str]):
     """

    mime_type: Union[Unset, str] = UNSET
    size_in_bytes: Union[Unset, int] = UNSET
    last_modified: Union[Unset, datetime.datetime] = UNSET
    expires: Union[Unset, datetime.datetime] = UNSET
    version_id: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        mime_type = self.mime_type
        size_in_bytes = self.size_in_bytes
        last_modified: Union[Unset, str] = UNSET
        if not isinstance(self.last_modified, Unset):
            last_modified = self.last_modified.isoformat()

        expires: Union[Unset, str] = UNSET
        if not isinstance(self.expires, Unset):
            expires = self.expires.isoformat()

        version_id = self.version_id

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if mime_type is not UNSET:
            field_dict["mime_type"] = mime_type
        if size_in_bytes is not UNSET:
            field_dict["size_in_bytes"] = size_in_bytes
        if last_modified is not UNSET:
            field_dict["last_modified"] = last_modified
        if expires is not UNSET:
            field_dict["expires"] = expires
        if version_id is not UNSET:
            field_dict["version_id"] = version_id

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        mime_type = d.pop("mime_type", UNSET)

        size_in_bytes = d.pop("size_in_bytes", UNSET)

        _last_modified = d.pop("last_modified", UNSET)
        last_modified: Union[Unset, datetime.datetime]
        if isinstance(_last_modified,  Unset):
            last_modified = UNSET
        else:
            last_modified = isoparse(_last_modified)




        _expires = d.pop("expires", UNSET)
        expires: Union[Unset, datetime.datetime]
        if isinstance(_expires,  Unset):
            expires = UNSET
        else:
            expires = isoparse(_expires)




        version_id = d.pop("version_id", UNSET)

        load_file_metadata_response_200 = cls(
            mime_type=mime_type,
            size_in_bytes=size_in_bytes,
            last_modified=last_modified,
            expires=expires,
            version_id=version_id,
        )

        load_file_metadata_response_200.additional_properties = d
        return load_file_metadata_response_200

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
