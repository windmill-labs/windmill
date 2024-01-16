from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast
from typing import Dict
from typing import cast, List

if TYPE_CHECKING:
  from ..models.multipart_file_upload_response_200_parts_item import MultipartFileUploadResponse200PartsItem





T = TypeVar("T", bound="MultipartFileUploadResponse200")


@_attrs_define
class MultipartFileUploadResponse200:
    """ 
        Attributes:
            upload_id (str):
            parts (List['MultipartFileUploadResponse200PartsItem']):
            is_done (bool):
     """

    upload_id: str
    parts: List['MultipartFileUploadResponse200PartsItem']
    is_done: bool
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.multipart_file_upload_response_200_parts_item import MultipartFileUploadResponse200PartsItem
        upload_id = self.upload_id
        parts = []
        for parts_item_data in self.parts:
            parts_item = parts_item_data.to_dict()

            parts.append(parts_item)




        is_done = self.is_done

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "upload_id": upload_id,
            "parts": parts,
            "is_done": is_done,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.multipart_file_upload_response_200_parts_item import MultipartFileUploadResponse200PartsItem
        d = src_dict.copy()
        upload_id = d.pop("upload_id")

        parts = []
        _parts = d.pop("parts")
        for parts_item_data in (_parts):
            parts_item = MultipartFileUploadResponse200PartsItem.from_dict(parts_item_data)



            parts.append(parts_item)


        is_done = d.pop("is_done")

        multipart_file_upload_response_200 = cls(
            upload_id=upload_id,
            parts=parts,
            is_done=is_done,
        )

        multipart_file_upload_response_200.additional_properties = d
        return multipart_file_upload_response_200

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
