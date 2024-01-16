from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..models.get_job_response_200_type import GetJobResponse200Type
from ..types import UNSET, Unset






T = TypeVar("T", bound="GetJobResponse200")


@_attrs_define
class GetJobResponse200:
    """ 
        Attributes:
            type (Union[Unset, GetJobResponse200Type]):
     """

    type: Union[Unset, GetJobResponse200Type] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        type: Union[Unset, str] = UNSET
        if not isinstance(self.type, Unset):
            type = self.type.value


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if type is not UNSET:
            field_dict["type"] = type

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        _type = d.pop("type", UNSET)
        type: Union[Unset, GetJobResponse200Type]
        if isinstance(_type,  Unset):
            type = UNSET
        else:
            type = GetJobResponse200Type(_type)




        get_job_response_200 = cls(
            type=type,
        )

        get_job_response_200.additional_properties = d
        return get_job_response_200

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
