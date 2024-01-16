from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from typing import cast, List
from typing import Dict
from ..types import UNSET, Unset

if TYPE_CHECKING:
  from ..models.multipart_file_upload_json_body_parts_item import MultipartFileUploadJsonBodyPartsItem





T = TypeVar("T", bound="MultipartFileUploadJsonBody")


@_attrs_define
class MultipartFileUploadJsonBody:
    """ 
        Attributes:
            file_key (str):
            parts (List['MultipartFileUploadJsonBodyPartsItem']):
            is_final (bool):
            cancel_upload (bool):
            part_content (Union[Unset, List[int]]):
            upload_id (Union[Unset, str]):
            s3_resource_path (Union[Unset, str]):
     """

    file_key: str
    parts: List['MultipartFileUploadJsonBodyPartsItem']
    is_final: bool
    cancel_upload: bool
    part_content: Union[Unset, List[int]] = UNSET
    upload_id: Union[Unset, str] = UNSET
    s3_resource_path: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.multipart_file_upload_json_body_parts_item import MultipartFileUploadJsonBodyPartsItem
        file_key = self.file_key
        parts = []
        for parts_item_data in self.parts:
            parts_item = parts_item_data.to_dict()

            parts.append(parts_item)




        is_final = self.is_final
        cancel_upload = self.cancel_upload
        part_content: Union[Unset, List[int]] = UNSET
        if not isinstance(self.part_content, Unset):
            part_content = self.part_content




        upload_id = self.upload_id
        s3_resource_path = self.s3_resource_path

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "file_key": file_key,
            "parts": parts,
            "is_final": is_final,
            "cancel_upload": cancel_upload,
        })
        if part_content is not UNSET:
            field_dict["part_content"] = part_content
        if upload_id is not UNSET:
            field_dict["upload_id"] = upload_id
        if s3_resource_path is not UNSET:
            field_dict["s3_resource_path"] = s3_resource_path

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.multipart_file_upload_json_body_parts_item import MultipartFileUploadJsonBodyPartsItem
        d = src_dict.copy()
        file_key = d.pop("file_key")

        parts = []
        _parts = d.pop("parts")
        for parts_item_data in (_parts):
            parts_item = MultipartFileUploadJsonBodyPartsItem.from_dict(parts_item_data)



            parts.append(parts_item)


        is_final = d.pop("is_final")

        cancel_upload = d.pop("cancel_upload")

        part_content = cast(List[int], d.pop("part_content", UNSET))


        upload_id = d.pop("upload_id", UNSET)

        s3_resource_path = d.pop("s3_resource_path", UNSET)

        multipart_file_upload_json_body = cls(
            file_key=file_key,
            parts=parts,
            is_final=is_final,
            cancel_upload=cancel_upload,
            part_content=part_content,
            upload_id=upload_id,
            s3_resource_path=s3_resource_path,
        )

        multipart_file_upload_json_body.additional_properties = d
        return multipart_file_upload_json_body

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
