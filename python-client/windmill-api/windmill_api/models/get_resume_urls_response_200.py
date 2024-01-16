from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset







T = TypeVar("T", bound="GetResumeUrlsResponse200")


@_attrs_define
class GetResumeUrlsResponse200:
    """ 
        Attributes:
            approval_page (str):
            resume (str):
            cancel (str):
     """

    approval_page: str
    resume: str
    cancel: str
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        approval_page = self.approval_page
        resume = self.resume
        cancel = self.cancel

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "approvalPage": approval_page,
            "resume": resume,
            "cancel": cancel,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        approval_page = d.pop("approvalPage")

        resume = d.pop("resume")

        cancel = d.pop("cancel")

        get_resume_urls_response_200 = cls(
            approval_page=approval_page,
            resume=resume,
            cancel=cancel,
        )

        get_resume_urls_response_200.additional_properties = d
        return get_resume_urls_response_200

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
