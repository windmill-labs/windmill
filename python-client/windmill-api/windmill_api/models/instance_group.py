from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset
from typing import cast, List






T = TypeVar("T", bound="InstanceGroup")


@_attrs_define
class InstanceGroup:
    """ 
        Attributes:
            name (str):
            summary (Union[Unset, str]):
            emails (Union[Unset, List[str]]):
     """

    name: str
    summary: Union[Unset, str] = UNSET
    emails: Union[Unset, List[str]] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        name = self.name
        summary = self.summary
        emails: Union[Unset, List[str]] = UNSET
        if not isinstance(self.emails, Unset):
            emails = self.emails





        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "name": name,
        })
        if summary is not UNSET:
            field_dict["summary"] = summary
        if emails is not UNSET:
            field_dict["emails"] = emails

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        name = d.pop("name")

        summary = d.pop("summary", UNSET)

        emails = cast(List[str], d.pop("emails", UNSET))


        instance_group = cls(
            name=name,
            summary=summary,
            emails=emails,
        )

        instance_group.additional_properties = d
        return instance_group

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
