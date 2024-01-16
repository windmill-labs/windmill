from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset
from ..models.load_file_preview_response_200_content_type import LoadFilePreviewResponse200ContentType






T = TypeVar("T", bound="LoadFilePreviewResponse200")


@_attrs_define
class LoadFilePreviewResponse200:
    """ 
        Attributes:
            content_type (LoadFilePreviewResponse200ContentType):
            msg (Union[Unset, str]):
            content (Union[Unset, str]):
            download_url (Union[Unset, str]):
     """

    content_type: LoadFilePreviewResponse200ContentType
    msg: Union[Unset, str] = UNSET
    content: Union[Unset, str] = UNSET
    download_url: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        content_type = self.content_type.value

        msg = self.msg
        content = self.content
        download_url = self.download_url

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "content_type": content_type,
        })
        if msg is not UNSET:
            field_dict["msg"] = msg
        if content is not UNSET:
            field_dict["content"] = content
        if download_url is not UNSET:
            field_dict["download_url"] = download_url

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        content_type = LoadFilePreviewResponse200ContentType(d.pop("content_type"))




        msg = d.pop("msg", UNSET)

        content = d.pop("content", UNSET)

        download_url = d.pop("download_url", UNSET)

        load_file_preview_response_200 = cls(
            content_type=content_type,
            msg=msg,
            content=content,
            download_url=download_url,
        )

        load_file_preview_response_200.additional_properties = d
        return load_file_preview_response_200

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
