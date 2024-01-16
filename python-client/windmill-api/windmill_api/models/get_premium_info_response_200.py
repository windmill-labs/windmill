from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="GetPremiumInfoResponse200")


@_attrs_define
class GetPremiumInfoResponse200:
    """ 
        Attributes:
            premium (bool):
            usage (Union[Unset, float]):
            seats (Union[Unset, float]):
     """

    premium: bool
    usage: Union[Unset, float] = UNSET
    seats: Union[Unset, float] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        premium = self.premium
        usage = self.usage
        seats = self.seats

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "premium": premium,
        })
        if usage is not UNSET:
            field_dict["usage"] = usage
        if seats is not UNSET:
            field_dict["seats"] = seats

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        premium = d.pop("premium")

        usage = d.pop("usage", UNSET)

        seats = d.pop("seats", UNSET)

        get_premium_info_response_200 = cls(
            premium=premium,
            usage=usage,
            seats=seats,
        )

        get_premium_info_response_200.additional_properties = d
        return get_premium_info_response_200

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
